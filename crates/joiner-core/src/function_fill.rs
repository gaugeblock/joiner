//! Function-level fill: one function in, one validated candidate out, pair preserved.
//!
//! `filler::HoleFiller` is the grammar-masked path: it needs a `DadoPattern` and a `LexicalScope` and
//! compiles a `batten` constraint per hole. This module is the unconstrained counterpart used by
//! `joiner fill` when there is no registered pattern for the code being changed - it asks for the
//! whole function, validates the syntax, and retries once.
//!
//! The retry is what produces `Gold` pairs (`GB-167`): the failed candidate is kept as
//! `rejected_attempt`, so a repaired fill is preference data rather than just a success.

use std::path::Path;

use gaugeblock_contracts::{InferenceRequest, ModelRole, ReasoningEffort, TelemetryTier};
use sawhorse_core::router::RoleRouter;

use crate::error::{JoinerError, Result};

/// Outcome of filling a single function.
#[derive(Debug, Clone, PartialEq)]
pub struct FillOutcome {
    /// The candidate that validated.
    pub code: String,
    /// `Silver` when the first attempt validated, `Gold` when a repair did.
    pub tier: TelemetryTier,
    /// The attempt that failed, present only on a `Gold` pair.
    pub rejected_attempt: Option<String>,
    /// How many completions were requested.
    pub attempts: u32,
}

/// Default token budget for one fill attempt.
///
/// Reasoning models spend budget on the thinking trace before emitting any content, so a tight
/// budget yields an empty completion and no candidate at all - which is exactly how `joiner fill`
/// produced nothing at 1024 tokens against a reasoning model (`GB-206`). 1024 was adequate for a
/// plain chat model and is not adequate for a reasoning one, so the default is sized for the latter
/// and can be lowered per invocation.
pub const DEFAULT_FILL_MAX_TOKENS: u16 = 8192;

/// How a candidate is accepted.
///
/// Syntax alone is a weak oracle: a hallucinated call parses cleanly, so nothing fails and no
/// preference pair is ever recorded (`GB-213`). Compiler verification makes the oracle able to fail,
/// at the cost of a build per attempt.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum VerifyMode {
    /// Parse the candidate only. Fast, and the default.
    #[default]
    Syntax,
    /// Parse it, then compile the containing crate with `miter verify`.
    Crate(std::path::PathBuf),
}

/// Knobs for [`fill_function_with`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillOptions {
    /// Maximum tokens per attempt, forwarded to the provider unchanged.
    pub max_tokens: u16,
    /// Acceptance oracle.
    pub verify: VerifyMode,
    /// Leave the validated candidate on disk (the `--apply` contract).
    pub apply: bool,
}

impl Default for FillOptions {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_FILL_MAX_TOKENS,
            verify: VerifyMode::Syntax,
            apply: false,
        }
    }
}

/// Extracts the byte range of `fn <symbol>` by brace matching.
///
/// A parser would be better, but the caller only needs a slice to substitute and the grammar check
/// afterwards decides whether the result is real code.
#[must_use]
pub fn function_range(source: &str, symbol: &str) -> Option<(usize, usize)> {
    let start = source.find(&format!("fn {symbol}"))?;
    let open = source[start..].find('{')? + start;

    let mut depth = 0usize;
    for (offset, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start, open + offset + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// Drops markdown code fences, keeping the fenced body.
#[must_use]
pub fn strip_code_fences(text: &str) -> String {
    let trimmed = text.trim();
    let Some(open) = trimmed.find("```") else {
        return trimmed.to_string();
    };
    let after_open = &trimmed[open + 3..];
    let body = after_open
        .find('\n')
        .map_or(after_open, |newline| &after_open[newline + 1..]);
    match body.rfind("```") {
        Some(close) => body[..close].trim_end().to_string(),
        None => body.trim_end().to_string(),
    }
}

/// `Ok(())` when the source parses with zero syntax errors, otherwise the diagnostics.
pub fn assess_rust(source: &str) -> std::result::Result<(), Vec<String>> {
    match miter_engine::polyglot::syntax_diagnostics(source, miter_engine::polyglot::Language::Rust)
    {
        Ok(diagnostics) if diagnostics.is_empty() => Ok(()),
        Ok(diagnostics) => Err(diagnostics
            .iter()
            .map(|d| format!("L{}:C{} {:?}", d.line, d.column, d.kind))
            .collect()),
        Err(error) => Err(vec![error.to_string()]),
    }
}

/// Fills `symbol` in `file`, or the whole file when no symbol is given.
///
/// One completion is requested; if its syntax does not validate, exactly one repair follows using the
/// diagnostics returned, and the rejected candidate is kept so a repaired fill becomes a `Gold` pair.
///
/// # Errors
///
/// [`JoinerError::Io`] when the file cannot be read, [`JoinerError::PatternNotFound`] when the symbol
/// is absent, [`JoinerError::Sawhorse`] when inference fails, and [`JoinerError::HoleFillFailed`] when
/// neither attempt validates.
pub async fn fill_function(
    router: &RoleRouter,
    file: &Path,
    symbol: Option<&str>,
) -> Result<FillOutcome> {
    fill_function_with(router, file, symbol, FillOptions::default()).await
}

/// Restores a file when dropped, so a candidate can be compiled in place without risking the tree.
struct RestoreGuard {
    path: std::path::PathBuf,
    original: String,
    keep: bool,
}

impl Drop for RestoreGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::write(&self.path, &self.original);
        }
    }
}

/// Compiles the crate at `crate_dir` with `miter verify` and returns the diagnostics if it fails.
fn verify_in_crate(crate_dir: &std::path::Path) -> std::result::Result<(), Vec<String>> {
    let output = std::process::Command::new("miter")
        .arg("verify")
        .arg("--path")
        .arg(crate_dir)
        .arg("--json")
        .output()
        .map_err(|error| vec![format!("miter verify could not run: {error}")])?;

    let body = String::from_utf8_lossy(&output.stdout);
    let parsed: Option<serde_json::Value> = serde_json::from_str(body.trim()).ok();
    let outcome = parsed
        .as_ref()
        .and_then(|value| value.get("outcome"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    if outcome == "success" {
        return Ok(());
    }

    let message = parsed
        .as_ref()
        .and_then(|value| value.get("message"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| String::from_utf8_lossy(&output.stderr).to_string());
    Err(vec![format!("compile: {message}")])
}

/// Writes `candidate` into `file`, compiles the crate, and restores the file unless `keep`.
fn compiles_in_place(
    file: &std::path::Path,
    candidate: &str,
    crate_dir: &std::path::Path,
    keep: bool,
) -> std::result::Result<(), Vec<String>> {
    let original = std::fs::read_to_string(file).unwrap_or_default();
    let mut guard = RestoreGuard {
        path: file.to_path_buf(),
        original,
        keep: false,
    };
    if let Err(error) = std::fs::write(file, candidate) {
        return Err(vec![format!("candidate could not be written: {error}")]);
    }
    let verdict = verify_in_crate(crate_dir);
    guard.keep = keep && verdict.is_ok();
    drop(guard);
    verdict
}

/// Fills `symbol` in `file` with an explicit token budget.
///
/// # Errors
///
/// As [`fill_function`].
/// Builds the hole-filler request. Pure so the token budget is testable without a provider.
#[must_use]
fn build_request(prompt: String, max_tokens: u16) -> InferenceRequest {
    InferenceRequest {
        role: ModelRole::HoleFiller,
        prompt,
        system_prompt: Some(
            "You are a precise Rust engineer. Return compiling, minimal code and nothing else."
                .to_string(),
        ),
        grammar_constraint: None,
        grammar_format: None,
        max_tokens,
        temperature: 0.2,
        // Filling a function is deterministic work, so the thinking trace is pure overhead: a
        // reasoning model will spend the whole budget there and return no content at all, which is
        // what emptied the Gold path at 1024 and again at 8192 on a longer prompt (GB-206, GB-212).
        // Disabling it is the fix; the large budget is headroom for the answer, not for the thinking.
        reasoning_effort: Some(ReasoningEffort::None),
    }
}

pub async fn fill_function_with(
    router: &RoleRouter,
    file: &Path,
    symbol: Option<&str>,
    options: FillOptions,
) -> Result<FillOutcome> {
    let source = std::fs::read_to_string(file)?;
    let context = match symbol {
        Some(name) => {
            let (start, end) = function_range(&source, name)
                .ok_or_else(|| JoinerError::PatternNotFound(format!("fn {name}")))?;
            source[start..end].to_string()
        }
        None => source.clone(),
    };

    let mut rejected: Option<String> = None;
    let mut diagnostics: Vec<String> = Vec::new();

    for attempt in 1..=2usize {
        let label = symbol.unwrap_or("the file");
        let diagnostic_block = if diagnostics.is_empty() {
            "none".to_string()
        } else {
            diagnostics.join("\n")
        };
        let prompt = format!(
            "Fill {label} in Rust.\n\nCurrent code:\n```rust\n{context}\n```\n\nSyntax diagnostics from the previous attempt:\n{diagnostic_block}\n\nReturn only the complete replacement code inside one ```rust fence."
        );

        let request = build_request(prompt, options.max_tokens);

        let response = router.complete(&request).await?;
        let candidate = strip_code_fences(&response.text);

        let syntax_ok = assess_rust(&candidate);
        let verdict = match (&syntax_ok, &options.verify) {
            (Ok(()), VerifyMode::Crate(crate_dir)) => {
                compiles_in_place(file, &candidate, crate_dir, options.apply)
            }
            (Ok(()), VerifyMode::Syntax) => Ok(()),
            (Err(diagnostics), _) => Err(diagnostics.clone()),
        };

        match verdict {
            Ok(()) => {
                let tier = if attempt == 1 {
                    TelemetryTier::Silver
                } else {
                    TelemetryTier::Gold
                };
                return Ok(FillOutcome {
                    code: candidate,
                    tier,
                    rejected_attempt: if attempt == 1 { None } else { rejected },
                    attempts: attempt as u32,
                });
            }
            Err(found) => {
                diagnostics = found;
                rejected = Some(candidate);
            }
        }
    }

    Err(JoinerError::HoleFillFailed {
        hole_id: symbol.unwrap_or("<file>").to_string(),
        reason: format!("no candidate validated: {}", diagnostics.join("; ")),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{DEFAULT_FILL_MAX_TOKENS, FillOptions, VerifyMode, build_request};
    use gaugeblock_contracts::ReasoningEffort;

    // Deliberate invariant on a constant: the budget must not regress below what a reasoning
    // model needs, and clippy flags constant assertions by design.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn test_default_budget_leaves_room_for_a_thinking_trace() {
        assert!(
            DEFAULT_FILL_MAX_TOKENS >= 8192,
            "a reasoning model spends budget before emitting content (GB-206)"
        );
        assert_eq!(FillOptions::default().max_tokens, DEFAULT_FILL_MAX_TOKENS);
    }

    #[test]
    fn test_the_budget_reaches_the_request_unchanged() {
        let request = build_request("fill this".to_string(), 4096);
        assert_eq!(request.max_tokens, 4096);
        assert_eq!(request.temperature, 0.2);
        assert!(request.system_prompt.is_some());
        assert!(request.grammar_constraint.is_none());
        assert_eq!(
            request.reasoning_effort,
            Some(ReasoningEffort::None),
            "the filler must not pay for a thinking trace (GB-212)"
        );
    }

    #[test]
    fn test_syntax_is_the_default_oracle_and_a_crate_can_be_named() {
        assert_eq!(FillOptions::default().verify, VerifyMode::Syntax);
        assert!(
            !FillOptions::default().apply,
            "nothing is written unless asked"
        );
        let explicit = FillOptions {
            verify: VerifyMode::Crate(std::path::PathBuf::from("/tmp/crate")),
            ..FillOptions::default()
        };
        assert_ne!(explicit.verify, VerifyMode::Syntax);
    }

    #[test]
    fn test_an_explicit_option_overrides_the_default() {
        let options = FillOptions {
            max_tokens: 512,
            ..FillOptions::default()
        };
        assert_eq!(options.max_tokens, 512);
        assert_ne!(options, FillOptions::default());
    }
}

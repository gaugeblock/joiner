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
    /// Reasoning effort to request; defaults to disabled because filling is deterministic work.
    pub reasoning_effort: Option<ReasoningEffort>,
}

impl Default for FillOptions {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_FILL_MAX_TOKENS,
            verify: VerifyMode::Syntax,
            apply: false,
            reasoning_effort: None,
        }
    }
}

/// Source surgery and candidate verification, split out to respect the file-size law.
pub(super) mod prepare;

use prepare::{build_prompt, compiles_in_place, declared_dependencies};

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
pub use prepare::{assess_rust, function_range, strip_code_fences};

/// Fills `symbol` in `file` with an explicit token budget.
///
/// # Errors
///
/// As [`fill_function`].
/// Builds the hole-filler request. Pure so the token budget is testable without a provider.
#[must_use]
fn build_request(
    prompt: String,
    max_tokens: u16,
    effort: Option<ReasoningEffort>,
) -> InferenceRequest {
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
        // Disabled by default: filling is deterministic work and a thinking trace spends the budget
        // before any content appears (GB-206, GB-212). A caller can raise it - the configured
        // provider effort wins - which is what diagnostics work needs (GB-215).
        reasoning_effort: Some(effort.unwrap_or(ReasoningEffort::None)),
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

    // Only meaningful when verification happens inside a crate: that is the only case where a
    // dependency the model invents can be checked against reality (GB-224).
    let available = match &options.verify {
        VerifyMode::Crate(dir) => declared_dependencies(dir),
        VerifyMode::Syntax => Vec::new(),
    };

    let mut rejected: Option<String> = None;
    let mut diagnostics: Vec<String> = Vec::new();

    for attempt in 1..=2usize {
        let label = symbol.unwrap_or("the file");
        let prompt = build_prompt(label, &context, &diagnostics, &available);

        let request = build_request(prompt, options.max_tokens, options.reasoning_effort);

        let response = router.complete(&request).await?;
        let candidate = strip_code_fences(&response.text);

        let syntax_ok = assess_rust(&candidate);
        // Verify what will be applied, not the bare body. The candidate is one function; writing it
        // as the whole file deletes every other item and every `mod` declaration, so a candidate that
        // correctly delegates to `crate::text` failed with "cannot find `text` in the crate root"
        // while `--apply` (which splices) would have produced a compiling file.
        let spliced = match symbol {
            Some(name) => match function_range(&source, name) {
                Some((start, end)) => {
                    format!("{}{}{}", &source[..start], candidate, &source[end..])
                }
                None => candidate.clone(),
            },
            None => candidate.clone(),
        };

        let verdict = match (&syntax_ok, &options.verify) {
            (Ok(()), VerifyMode::Crate(crate_dir)) => {
                compiles_in_place(file, &spliced, crate_dir, options.apply)
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
#[path = "function_fill_tests.rs"]
mod tests;

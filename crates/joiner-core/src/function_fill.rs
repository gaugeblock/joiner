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

use gaugeblock_contracts::{InferenceRequest, ModelRole, TelemetryTier};
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

        let request = InferenceRequest {
            role: ModelRole::HoleFiller,
            prompt,
            system_prompt: Some(
                "You are a precise Rust engineer. Return compiling, minimal code and nothing else."
                    .to_string(),
            ),
            grammar_constraint: None,
            grammar_format: None,
            max_tokens: 1024,
            temperature: 0.2,
            reasoning_effort: None,
        };

        let response = router.complete(&request).await?;
        let candidate = strip_code_fences(&response.text);

        match assess_rust(&candidate) {
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

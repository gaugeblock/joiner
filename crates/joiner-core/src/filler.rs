//! Batten-constrained parallel and sequential hole filling loop.

use crate::error::{JoinerError, Result};
use batten_core::model::{GrammarFormat, LexicalScope};
use batten_core::{compile_grammar, filter_scope_for_hole, validate_candidate};
use dado_core::{DadoPattern, TypedHole};
use futures::future::join_all;
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::{InferenceRequest, ModelRole};
use std::collections::BTreeMap;
use tracing::{debug, warn};

/// Context parameters driving hole generation or localized repair.
#[derive(Debug, Clone)]
pub struct HoleFillRequest<'a> {
    /// Target pattern being populated.
    pub pattern: &'a DadoPattern,
    /// In-scope variables and imports at the splice site.
    pub scope: &'a LexicalScope,
    /// Whether independent holes are executed concurrently.
    pub parallel: bool,
    /// Optional existing fills when performing targeted repair.
    pub previous_fills: Option<&'a BTreeMap<String, String>>,
    /// Specific hole ID to regenerate (targeted repair mode).
    pub target_hole_id: Option<&'a str>,
    /// Located compiler error message driving the repair prompt.
    pub diagnostic_feedback: Option<&'a str>,
}

/// Orchestrates batten grammar compilation, LLM sampling, and candidate validation.
#[derive(Debug, Clone)]
pub struct HoleFiller {
    role_router: std::sync::Arc<RoleRouter>,
}

impl HoleFiller {
    /// Creates a new hole filler engine.
    pub fn new(role_router: std::sync::Arc<RoleRouter>) -> Self {
        Self { role_router }
    }

    /// Fills all (or a targeted subset of) typed holes in a pattern.
    pub async fn fill(&self, req: HoleFillRequest<'_>) -> Result<BTreeMap<String, String>> {
        let mut results = req.previous_fills.cloned().unwrap_or_default();

        let holes_to_fill: Vec<&TypedHole> = req
            .pattern
            .holes
            .iter()
            .filter(|h| match req.target_hole_id {
                Some(target) => h.hole_id == target,
                None => true,
            })
            .collect();

        if holes_to_fill.is_empty() {
            return Ok(results);
        }

        if req.parallel && req.target_hole_id.is_none() && holes_to_fill.len() > 1 {
            // Concurrent execution path
            let futures = holes_to_fill.into_iter().map(|hole| {
                self.fill_single_hole(
                    hole,
                    req.scope,
                    req.pattern.language.name(),
                    req.diagnostic_feedback,
                )
            });
            let filled = join_all(futures).await;
            for res in filled {
                let (hole_id, candidate) = res?;
                results.insert(hole_id, candidate);
            }
        } else {
            // Sequential execution path
            for hole in holes_to_fill {
                let (hole_id, candidate) = self
                    .fill_single_hole(
                        hole,
                        req.scope,
                        req.pattern.language.name(),
                        req.diagnostic_feedback,
                    )
                    .await?;
                results.insert(hole_id, candidate);
            }
        }

        Ok(results)
    }

    async fn fill_single_hole(
        &self,
        hole: &TypedHole,
        scope: &LexicalScope,
        lang: &str,
        diagnostic: Option<&str>,
    ) -> Result<(String, String)> {
        debug!(hole_id = %hole.hole_id, expected = %hole.expected_type, "Filling hole");

        let filtered = filter_scope_for_hole(scope, hole);
        let constraint = if !filtered.allowed_identifiers.is_empty() {
            compile_grammar(&filtered, hole, GrammarFormat::Gbnf).ok()
        } else {
            None
        };

        let mut prompt = format!(
            "Generate an expression for typed hole '{}' in {}.\nExpected type: `{}`\nAST kind: `{}`\n",
            hole.hole_id, lang, hole.expected_type, hole.ast_node_kind
        );

        if !filtered.allowed_identifiers.is_empty() {
            prompt.push_str(&format!(
                "Allowed identifiers: {}\n",
                filtered.allowed_identifiers.join(", ")
            ));
        }

        if let Some(err) = diagnostic {
            prompt.push_str(&format!(
                "\nPrevious attempt failed compiler verification with:\n{}\nFix the expression.\n",
                err
            ));
        }

        let sys_prompt = format!(
            "You are a grammar-masked code generator. Output ONLY the raw expression for hole '{}'. Zero markdown, zero formatting backticks, zero explanations.",
            hole.hole_id
        );

        let req = InferenceRequest {
            role: ModelRole::HoleFiller,
            prompt,
            system_prompt: Some(sys_prompt),
            grammar_constraint: constraint.as_ref().map(|c| c.grammar_definition.clone()),
            grammar_format: constraint.as_ref().map(|_| sawhorse_core::types::GrammarFormat::Gbnf),
            max_tokens: hole.max_tokens,
            temperature: if diagnostic.is_some() { 0.2 } else { 0.05 },
        };

        // Up to 2 validation retry attempts
        for attempt in 1..=2 {
            let resp = self
                .role_router
                .complete(&req)
                .await
                .map_err(JoinerError::from)?;
            let candidate = clean_candidate(&resp.text);

            if let Some(c) = &constraint {
                let report = validate_candidate(
                    &candidate,
                    c,
                    hole.allow_literals,
                    Some(hole.max_tokens),
                );
                if !report.is_valid {
                    warn!(
                        hole = %hole.hole_id,
                        attempt,
                        violations = ?report.violations,
                        "Candidate failed batten validation"
                    );
                    if attempt == 2 {
                        // Return anyway so compiler diagnostics can give definitive feedback
                        return Ok((hole.hole_id.clone(), candidate));
                    }
                    continue;
                }
            }

            return Ok((hole.hole_id.clone(), candidate));
        }

        Err(JoinerError::HoleFillFailed {
            hole_id: hole.hole_id.clone(),
            reason: "Exhausted validation attempts".into(),
        })
    }
}

fn clean_candidate(raw: &str) -> String {
    let trimmed = raw.trim();
    let unquoted = trimmed.strip_prefix("```").unwrap_or(trimmed);
    let stripped = unquoted
        .strip_prefix("rust\n")
        .or_else(|| unquoted.strip_prefix("typescript\n"))
        .unwrap_or(unquoted);
    stripped.strip_suffix("```").unwrap_or(stripped).trim().to_string()
}

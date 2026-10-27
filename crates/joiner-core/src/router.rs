//! System 1 pattern, surgical edit, and Oracle strategy router.

use crate::error::{JoinerError, Result};
use crate::model::SynthesisStrategy;
use dado_core::PatternRegistry;
use miter_engine::Language;
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::{InferenceRequest, ModelRole};
use serde::Deserialize;
use std::sync::Arc;
use storypole_core::model::task::TaskNode;
use tracing::{debug, info};

/// Router response payload for structured LLM routing.
#[derive(Debug, Deserialize)]
struct RouterDecisionPayload {
    action: String,
    pattern_id: Option<String>,
    mixins: Option<Vec<String>>,
    target_symbol: Option<String>,
    directive: Option<String>,
    rationale: Option<String>,
}

/// System 1 Router mapping `TaskNode` and scope to a deterministic `SynthesisStrategy`.
#[derive(Debug, Clone)]
pub struct System1Router {
    registry: Arc<PatternRegistry>,
    role_router: Option<Arc<RoleRouter>>,
}

impl System1Router {
    /// Creates a new router with the given pattern registry and optional LLM router.
    pub fn new(registry: Arc<PatternRegistry>, role_router: Option<Arc<RoleRouter>>) -> Self {
        Self {
            registry,
            role_router,
        }
    }

    /// Resolves the optimal synthesis strategy for a task.
    pub async fn route(&self, task: &TaskNode, lang: Language) -> Result<SynthesisStrategy> {
        debug!(ticket = %task.ticket_key, title = %task.title, "Routing task strategy");

        // Stage 1: Deterministic lookup via explicit task labels
        for label in &task.scope.labels {
            if let Some(pat_id) = label.strip_prefix("pattern:")
                && self.registry.get_pattern(pat_id).is_some()
            {
                let mixins = self.extract_mixins_from_labels(&task.scope.labels);
                info!(pattern_id = %pat_id, "Matched pattern via explicit task label");
                return Ok(SynthesisStrategy::PatternMatch {
                    pattern_id: pat_id.to_string(),
                    mixins,
                });
            }
        }

        // Search registry for candidate patterns matching language and title/slug query
        let query_candidates = self.find_candidates(task, lang);
        if query_candidates.len() == 1 {
            let pat_id = query_candidates[0].pattern_id.clone();
            let mixins = self.extract_mixins_from_labels(&task.scope.labels);
            info!(pattern_id = %pat_id, "Matched single unambiguous pattern via title/slug");
            return Ok(SynthesisStrategy::PatternMatch {
                pattern_id: pat_id,
                mixins,
            });
        }

        // Stage 2: Ambiguous or multi-candidate — invoke sawhorse System1Router if available
        if let Some(router) = &self.role_router
            && let Ok(strategy) = self
                .route_with_llm(task, lang, &query_candidates, router)
                .await
        {
            return Ok(strategy);
        }

        // If candidates existed but LLM unavailable, select top candidate if available
        if let Some(first) = query_candidates.first() {
            let mixins = self.extract_mixins_from_labels(&task.scope.labels);
            return Ok(SynthesisStrategy::PatternMatch {
                pattern_id: first.pattern_id.clone(),
                mixins,
            });
        }

        // Stage 3: Surgical edit fallback if target symbols exist, otherwise Oracle
        if let Some(target_symbol) = task.scope.target_symbols.first() {
            info!(symbol = %target_symbol, "Routing to AST surgical edit");
            Ok(SynthesisStrategy::SurgicalEdit {
                target_symbol: target_symbol.clone(),
                action: task.title.clone(),
            })
        } else {
            info!("No pattern matched; escalating to unconstrained Oracle fallback");
            Ok(SynthesisStrategy::OracleFallback {
                rationale: format!("No pattern match for task: {}", task.title),
            })
        }
    }

    fn find_candidates(&self, task: &TaskNode, lang: Language) -> Vec<&dado_core::DadoPattern> {
        let slug_query = task.slug.replace('-', " ");
        let mut list = self.registry.list_patterns(Some(lang), Some(&task.title));
        if list.is_empty() {
            list = self.registry.list_patterns(Some(lang), Some(&slug_query));
        }
        if list.is_empty() {
            for tag in &task.scope.labels {
                let tag_matches = self.registry.list_patterns(Some(lang), Some(tag));
                for m in tag_matches {
                    if !list.iter().any(|p| p.pattern_id == m.pattern_id) {
                        list.push(m);
                    }
                }
            }
        }
        list
    }

    fn extract_mixins_from_labels(&self, labels: &[String]) -> Vec<String> {
        labels
            .iter()
            .filter_map(|l| l.strip_prefix("mixin:"))
            .filter(|id| self.registry.get_mixin(id).is_some())
            .map(|s| s.to_string())
            .collect()
    }

    async fn route_with_llm(
        &self,
        task: &TaskNode,
        _lang: Language,
        candidates: &[&dado_core::DadoPattern],
        router: &RoleRouter,
    ) -> Result<SynthesisStrategy> {
        let cand_str = candidates
            .iter()
            .take(5)
            .map(|p| format!("- `{}`: {}", p.pattern_id, p.description))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Select strategy for task '{}': {}\nCandidates:\n{}\nRespond JSON: {{\"action\":\"pattern\"|\"surgical_edit\"|\"oracle\", \"pattern_id\": \"...\", \"mixins\": []}}",
            task.ticket_key, task.description, cand_str
        );

        let req = InferenceRequest {
            role: ModelRole::System1Router,
            prompt,
            system_prompt: Some(
                "You are a deterministic AST strategy classifier. Output JSON.".into(),
            ),
            grammar_constraint: None,
            grammar_format: None,
            max_tokens: 150,
            temperature: 0.0,
            reasoning_effort: None,
        };

        let resp = router.complete(&req).await.map_err(JoinerError::from)?;
        let parsed: RouterDecisionPayload = serde_json::from_str(&resp.text)?;

        match parsed.action.as_str() {
            "pattern" if parsed.pattern_id.is_some() => {
                let pid = parsed
                    .pattern_id
                    .ok_or_else(|| JoinerError::Configuration("missing pattern_id".into()))?;
                Ok(SynthesisStrategy::PatternMatch {
                    pattern_id: pid,
                    mixins: parsed.mixins.unwrap_or_default(),
                })
            }
            "surgical_edit" => Ok(SynthesisStrategy::SurgicalEdit {
                target_symbol: parsed.target_symbol.unwrap_or_else(|| "unknown".into()),
                action: parsed.directive.unwrap_or_else(|| task.title.clone()),
            }),
            _ => Ok(SynthesisStrategy::OracleFallback {
                rationale: parsed
                    .rationale
                    .unwrap_or_else(|| "Router selected oracle".into()),
            }),
        }
    }
}

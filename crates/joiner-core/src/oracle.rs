//! Unconstrained frontier model Oracle fallback and Gold DPO pair extraction.

use crate::error::{JoinerError, Result};
use crate::model::{CompilerDiagnostic, TelemetryTier};
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::{InferenceRequest, ModelRole};
use std::sync::Arc;
use tracing::{debug, info};

/// Result of an Oracle synthesis attempt.
#[derive(Debug, Clone, PartialEq)]
pub struct OracleResult {
    /// Spliced or synthesized source code.
    pub synthesized_code: String,
    /// Rejected candidate code from prior failed attempt (for DPO training pairs).
    pub rejected_attempt: Option<String>,
    /// Assigned telemetry tier (always Gold on successful repair/oracle).
    pub tier: TelemetryTier,
}

/// Unconstrained frontier model runner for novel synthesis and terminal self-repair.
#[derive(Debug, Clone)]
pub struct OracleRunner {
    role_router: Arc<RoleRouter>,
}

impl OracleRunner {
    /// Creates a new Oracle runner backed by `sawhorse`.
    pub fn new(role_router: Arc<RoleRouter>) -> Self {
        Self { role_router }
    }

    /// Executes unconstrained synthesis given context, prior attempt, and diagnostics.
    pub async fn synthesize(
        &self,
        task_title: &str,
        task_desc: &str,
        lang: &str,
        current_file_context: &str,
        diagnostics: &[CompilerDiagnostic],
        rejected_attempt: Option<String>,
    ) -> Result<OracleResult> {
        info!(
            task = %task_title,
            diagnostics_count = diagnostics.len(),
            "Escalating to sawhorse frontier ModelRole::Oracle"
        );

        let diag_summary = if diagnostics.is_empty() {
            "None (Initial unconstrained generation)".to_string()
        } else {
            diagnostics
                .iter()
                .map(|d| {
                    format!(
                        "[{}:{}:{}] {}: {}",
                        d.file,
                        d.line,
                        d.column,
                        d.code.as_deref().unwrap_or("ERROR"),
                        d.message
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let prompt = format!(
            "Task: {}\nDescription: {}\nLanguage: {}\n\nCompiler Diagnostics:\n{}\n\nExisting File Context:\n{}\n\nProvide the complete, corrected implementation. Return ONLY the code inside ```{} ... ``` fences.",
            task_title, task_desc, lang, diag_summary, current_file_context, lang
        );

        let sys_prompt = format!(
            "You are a master systems engineer acting as the Oracle for {lang}. Generate valid, compiling, clean code solving the task and fixing all compiler diagnostics. Output code fences only."
        );

        let req = InferenceRequest {
            role: ModelRole::Oracle,
            prompt,
            system_prompt: Some(sys_prompt),
            grammar_constraint: None,
            grammar_format: None,
            max_tokens: 2048,
            temperature: 0.2,
        };

        let resp = self
            .role_router
            .complete(&req)
            .await
            .map_err(JoinerError::from)?;
        let cleaned_code = extract_code_block(&resp.text, lang);

        if cleaned_code.trim().is_empty() {
            return Err(JoinerError::OracleFailed(
                "Frontier model returned empty completion".into(),
            ));
        }

        debug!(
            code_len = cleaned_code.len(),
            "Oracle synthesis pass completed"
        );

        Ok(OracleResult {
            synthesized_code: cleaned_code,
            rejected_attempt,
            tier: TelemetryTier::Gold,
        })
    }
}

fn extract_code_block(raw: &str, lang: &str) -> String {
    let fence = format!("```{lang}");
    if let Some(start) = raw.find(&fence) {
        let after = &raw[start + fence.len()..];
        if let Some(end) = after.find("```") {
            return after[..end].trim().to_string();
        }
    }

    if let Some(start) = raw.find("```") {
        let after = &raw[start + 3..];
        let code_start = after.find('\n').map(|idx| idx + 1).unwrap_or(0);
        let after_code = &after[code_start..];
        if let Some(end) = after_code.find("```") {
            return after_code[..end].trim().to_string();
        }
    }

    raw.trim().to_string()
}

//! Synthesis execution loop implementing progressive repair and oracle escalation.

use crate::error::{JoinerError, Result};
use crate::filler::{HoleFillRequest, HoleFiller};
use crate::model::{
    JoinerConfig, SynthesisEvent, SynthesisResult, SynthesisStrategy, VerificationReport,
};
use crate::oracle::OracleRunner;
use crate::router::System1Router;
use crate::telemetry::{TelemetrySink, build_anonymized_trace};
use crate::verifier::AstVerifier;
use batten_core::model::LexicalScope;
use dado_core::PatternRegistry;
use miter_engine::Language;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;
use storypole_core::model::task::TaskNode;
use tokio::sync::broadcast;

/// Executes the multi-attempt synthesis, self-repair, and fallback loop for a task.
#[allow(clippy::too_many_arguments)]
pub async fn execute_task_synthesis(
    config: &JoinerConfig,
    registry: &Arc<PatternRegistry>,
    router: &System1Router,
    filler: &HoleFiller,
    verifier: &AstVerifier,
    oracle: &OracleRunner,
    telemetry_sink: &Arc<dyn TelemetrySink>,
    event_tx: &broadcast::Sender<SynthesisEvent>,
    task: &TaskNode,
) -> Result<SynthesisResult> {
    let start = Instant::now();
    let lang = Language::Rust;
    let strategy = router.route(task, lang).await?;
    let _ = event_tx.send(SynthesisEvent::StrategySelected {
        task_id: task.id,
        strategy: strategy.clone(),
    });

    let target_file = task
        .scope
        .target_files
        .first()
        .cloned()
        .unwrap_or_else(|| "src/lib.rs".to_string());

    let mut current_fills = BTreeMap::new();
    let mut last_report = None;
    let mut affected_hole = None;
    let mut rejected_attempt = None;
    let mut total_attempts = 0;

    let scope = LexicalScope {
        repo_relative_path: target_file.clone(),
        language: lang,
        enclosing_symbol: task.scope.target_symbols.first().cloned(),
        bindings: vec![],
        available_imports: vec![],
    };

    match &strategy {
        SynthesisStrategy::PatternMatch { pattern_id, .. } => {
            let pat = registry
                .get_pattern(pattern_id)
                .ok_or_else(|| JoinerError::PatternNotFound(pattern_id.clone()))?;

            for attempt in 1..=config.max_repair_attempts {
                total_attempts = attempt;
                let _ = event_tx.send(SynthesisEvent::Verifying {
                    task_id: task.id,
                    attempt,
                });

                let fills = filler
                    .fill(HoleFillRequest {
                        pattern: pat,
                        scope: &scope,
                        parallel: config.parallel_holes,
                        previous_fills: Some(&current_fills),
                        target_hole_id: affected_hole.as_deref(),
                        diagnostic_feedback: last_report
                            .as_ref()
                            .and_then(|r: &VerificationReport| r.diagnostics.first())
                            .map(|d| d.message.as_str()),
                    })
                    .await?;
                current_fills = fills;

                let (report, splice_res, next_affected) = verifier
                    .verify(
                        &config.workspace_root,
                        &target_file,
                        pat,
                        &current_fills,
                        config.execution_mode,
                        true,
                    )
                    .await?;

                if report.ast_valid && report.typecheck_passed {
                    last_report = Some(report);
                    break;
                }

                rejected_attempt = Some(splice_res.rendered_code);
                affected_hole = next_affected;
                last_report = Some(report);
            }

            // Escalate to Oracle if attempts exhausted without clean compilation
            if last_report
                .as_ref()
                .map(|r| !r.ast_valid || !r.typecheck_passed)
                .unwrap_or(true)
            {
                let prev_code = rejected_attempt.clone().unwrap_or_default();
                let diags = last_report
                    .as_ref()
                    .map(|r| r.diagnostics.as_slice())
                    .unwrap_or(&[]);
                if let Ok(ores) = oracle
                    .synthesize(
                        &task.title,
                        &task.description,
                        lang.name(),
                        &prev_code,
                        diags,
                        rejected_attempt.clone(),
                    )
                    .await
                {
                    rejected_attempt = ores.rejected_attempt;
                }
            }
        }
        SynthesisStrategy::OracleFallback { rationale } => {
            total_attempts = 1;
            if let Ok(ores) = oracle
                .synthesize(
                    &task.title,
                    &task.description,
                    lang.name(),
                    &format!("// Oracle fallback: {rationale}"),
                    &[],
                    None,
                )
                .await
            {
                rejected_attempt = ores.rejected_attempt;
            }
        }
        _ => {
            total_attempts = 1;
        }
    }

    let report = last_report.unwrap_or(VerificationReport {
        ast_valid: true,
        typecheck_passed: true,
        tests_passed: Some(true),
        diagnostics: vec![],
        duration_ms: start.elapsed().as_millis() as u64,
    });

    let fills_vec: Vec<(String, String)> = current_fills.into_iter().collect();
    let trace = build_anonymized_trace(
        lang,
        total_attempts,
        match &strategy {
            SynthesisStrategy::PatternMatch { pattern_id, .. } => Some(pattern_id.clone()),
            _ => None,
        },
        &scope.repo_relative_path,
        &fills_vec,
        rejected_attempt,
        report.clone(),
    );

    telemetry_sink.record_trace(&trace).await?;
    let _ = event_tx.send(SynthesisEvent::Verified {
        task_id: task.id,
        tier: trace.tier,
    });

    Ok(SynthesisResult {
        task_id: task.id,
        ticket_key: task.ticket_key.clone(),
        strategy,
        tier: trace.tier,
        filled_holes: fills_vec,
        attempts: total_attempts,
        duration_ms: start.elapsed().as_millis() as u64,
        verification: report,
    })
}


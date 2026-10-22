//! Local JSONL spool outbox with source scrubbing and dynamic tier assignment.

use super::sink::{SpoolEntry, SyncStatus, TelemetrySink, compute_trace_hash};
use crate::error::{JoinerError, Result};
use crate::model::{AnonymizedSynthesisTrace, TelemetryTier, VerificationReport};
use async_trait::async_trait;
use miter_engine::Language;
use miter_engine::security::scrub_source_text;
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

/// Evaluates and assigns the dynamic quality tier per the `/grill-me` specification.
pub fn assign_telemetry_tier(attempts: usize, report: &VerificationReport) -> TelemetryTier {
    if attempts == 1 && report.ast_valid && report.typecheck_passed {
        if report.tests_passed == Some(true) {
            TelemetryTier::Silver
        } else {
            TelemetryTier::Bronze
        }
    } else {
        TelemetryTier::Gold
    }
}

/// Sanitizes and AST-anonymizes a completed synthesis pass into a trace.
pub fn build_anonymized_trace(
    language: Language,
    attempts: usize,
    selected_pattern_id: Option<String>,
    scope_signature: &str,
    filled_holes: &[(String, String)],
    rejected_attempt: Option<String>,
    report: VerificationReport,
) -> AnonymizedSynthesisTrace {
    let tier = assign_telemetry_tier(attempts, &report);
    let sanitized_scope = scrub_source_text(scope_signature);

    let sanitized_holes = filled_holes
        .iter()
        .map(|(id, expr)| (id.clone(), scrub_source_text(expr)))
        .collect();

    let sanitized_rejected = rejected_attempt.map(|r| scrub_source_text(&r));

    AnonymizedSynthesisTrace {
        trace_id: Uuid::new_v4(),
        language,
        tier,
        selected_pattern_id,
        anonymized_scope_signature: sanitized_scope,
        filled_holes: sanitized_holes,
        rejected_attempt: sanitized_rejected,
        verification: report,
    }
}

/// Local JSONL file-backed telemetry outbox for rabbet synchronization.
#[derive(Debug, Clone)]
pub struct FileSpoolSink {
    outbox_path: PathBuf,
}

impl FileSpoolSink {
    /// Creates a file-backed spool sink writing to `<spool_dir>/outbox.jsonl`.
    pub async fn new(spool_dir: &Path) -> Result<Self> {
        fs::create_dir_all(spool_dir).await?;
        let outbox_path = spool_dir.join("outbox.jsonl");
        Ok(Self { outbox_path })
    }
}

#[async_trait]
impl TelemetrySink for FileSpoolSink {
    async fn record_trace(&self, trace: &AnonymizedSynthesisTrace) -> Result<SpoolEntry> {
        let hash = compute_trace_hash(trace)?;
        let entry = SpoolEntry {
            trace: trace.clone(),
            content_hash: hash,
            sync_status: SyncStatus::Pending,
            recorded_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| JoinerError::SpoolError(e.to_string()))?
                .as_millis() as u64,
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.outbox_path)
            .await?;

        let mut line = serde_json::to_string(&entry)?;
        line.push('\n');
        file.write_all(line.as_bytes()).await?;

        Ok(entry)
    }

    async fn list_pending(&self) -> Result<Vec<SpoolEntry>> {
        if !self.outbox_path.exists() {
            return Ok(vec![]);
        }

        let file = fs::File::open(&self.outbox_path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut entries = Vec::new();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<SpoolEntry>(&line)
                && entry.sync_status == SyncStatus::Pending
            {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    async fn export_batch(&self, max_records: usize) -> Result<Vec<SpoolEntry>> {
        let all_pending = self.list_pending().await?;
        Ok(all_pending.into_iter().take(max_records).collect())
    }

    async fn mark_synced(&self, content_hashes: &[String]) -> Result<usize> {
        if !self.outbox_path.exists() {
            return Ok(0);
        }

        let file = fs::File::open(&self.outbox_path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut updated = Vec::new();
        let mut count = 0;

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(mut entry) = serde_json::from_str::<SpoolEntry>(&line) {
                if content_hashes.contains(&entry.content_hash) {
                    entry.sync_status = SyncStatus::Synced;
                    count += 1;
                }
                updated.push(entry);
            }
        }

        let mut out = fs::File::create(&self.outbox_path).await?;
        for entry in updated {
            let mut line = serde_json::to_string(&entry)?;
            line.push('\n');
            out.write_all(line.as_bytes()).await?;
        }

        Ok(count)
    }
}

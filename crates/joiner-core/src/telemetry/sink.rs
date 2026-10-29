//! TelemetrySink trait and in-memory/hub sink implementations.

use crate::error::{JoinerError, Result};
use crate::model::AnonymizedSynthesisTrace;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

// The outbox record and its sync state live in `gaugeblock-contracts` (GB-182) so `storypole`,
// `joiner` and `rabbet` read and write one format. Re-exported for every existing path.
pub use gaugeblock_contracts::{SpoolEntry, SyncStatus};

/// Pluggable telemetry sink interface for local spooling and remote hub sync.
#[async_trait]
pub trait TelemetrySink: Send + Sync + std::fmt::Debug {
    /// Records an anonymized trace into the sink outbox.
    async fn record_trace(&self, trace: &AnonymizedSynthesisTrace) -> Result<SpoolEntry>;

    /// Lists pending outbox entries that have not been synced yet.
    async fn list_pending(&self) -> Result<Vec<SpoolEntry>>;

    /// Exports up to `max_records` pending entries for batch dispatch.
    async fn export_batch(&self, max_records: usize) -> Result<Vec<SpoolEntry>>;

    /// Marks the given content hashes as synchronized.
    async fn mark_synced(&self, content_hashes: &[String]) -> Result<usize>;
}

/// Calculates the SHA-256 content hash of an anonymized trace.
pub fn compute_trace_hash(trace: &AnonymizedSynthesisTrace) -> Result<String> {
    SpoolEntry::hash_trace(trace).map_err(JoinerError::Json)
}

/// In-memory sink implementation for testing and ephemeral execution.
#[derive(Debug, Default, Clone)]
pub struct MemoryTelemetrySink {
    entries: Arc<RwLock<Vec<SpoolEntry>>>,
}

impl MemoryTelemetrySink {
    /// Creates an empty in-memory telemetry sink.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TelemetrySink for MemoryTelemetrySink {
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

        let mut lock = self.entries.write().await;
        lock.push(entry.clone());
        Ok(entry)
    }

    async fn list_pending(&self) -> Result<Vec<SpoolEntry>> {
        let lock = self.entries.read().await;
        Ok(lock
            .iter()
            .filter(|e| e.sync_status == SyncStatus::Pending)
            .cloned()
            .collect())
    }

    async fn export_batch(&self, max_records: usize) -> Result<Vec<SpoolEntry>> {
        let lock = self.entries.read().await;
        Ok(lock
            .iter()
            .filter(|e| e.sync_status == SyncStatus::Pending)
            .take(max_records)
            .cloned()
            .collect())
    }

    async fn mark_synced(&self, content_hashes: &[String]) -> Result<usize> {
        let mut count = 0;
        let mut lock = self.entries.write().await;
        for entry in lock.iter_mut() {
            if content_hashes.contains(&entry.content_hash) {
                entry.sync_status = SyncStatus::Synced;
                count += 1;
            }
        }
        Ok(count)
    }
}

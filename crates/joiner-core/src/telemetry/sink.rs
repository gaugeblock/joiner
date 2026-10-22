//! TelemetrySink trait and in-memory/hub sink implementations.

use crate::error::{JoinerError, Result};
use crate::model::AnonymizedSynthesisTrace;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Synchronization status of a telemetry trace in the outbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// Awaiting transmission to rabbet hub.
    #[default]
    Pending,
    /// Successfully synchronized with remote hub.
    Synced,
    /// Rejected by schema validator or hub deduplicator.
    Rejected,
}

/// A serialized telemetry entry tracked in the local outbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpoolEntry {
    /// The anonymized trace payload.
    pub trace: AnonymizedSynthesisTrace,
    /// Deterministic SHA-256 hash of the canonical trace JSON.
    pub content_hash: String,
    /// Current synchronization state.
    pub sync_status: SyncStatus,
    /// Timestamp in epoch milliseconds when spooled.
    pub recorded_at_ms: u64,
}

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
    let json = serde_json::to_vec(trace)?;
    let mut hasher = Sha256::new();
    hasher.update(&json);
    Ok(format!("{:x}", hasher.finalize()))
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

//! Telemetry module for AST-anonymized traces and rabbet spool synchronization.

pub mod sink;
pub mod spool;

pub use sink::{MemoryTelemetrySink, SpoolEntry, SyncStatus, TelemetrySink, compute_trace_hash};
pub use spool::{FileSpoolSink, assign_telemetry_tier, build_anonymized_trace};

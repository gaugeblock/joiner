#![allow(clippy::expect_used, clippy::unwrap_used)]

use joiner_core::model::{TelemetryTier, VerificationReport};
use joiner_core::telemetry::{
    FileSpoolSink, MemoryTelemetrySink, SyncStatus, TelemetrySink, assign_telemetry_tier,
    build_anonymized_trace, compute_trace_hash,
};
use miter_engine::Language;
use tempfile::tempdir;

#[test]
fn test_dynamic_tier_assignment() {
    let mut report = VerificationReport {
        ast_valid: true,
        typecheck_passed: true,
        tests_passed: Some(true),
        diagnostics: vec![],
        duration_ms: 120,
    };

    // Attempt 1 with unit tests passing -> Silver
    assert_eq!(assign_telemetry_tier(1, &report), TelemetryTier::Silver);

    // Attempt 1 without tests -> Bronze
    report.tests_passed = None;
    assert_eq!(assign_telemetry_tier(1, &report), TelemetryTier::Bronze);

    // Attempt 2 (repaired) -> Gold
    assert_eq!(assign_telemetry_tier(2, &report), TelemetryTier::Gold);

    // Attempt 3 / Oracle -> Gold
    assert_eq!(assign_telemetry_tier(3, &report), TelemetryTier::Gold);
}

#[tokio::test]
async fn test_anonymized_trace_scrubbing_and_hash() {
    let report = VerificationReport {
        ast_valid: true,
        typecheck_passed: true,
        tests_passed: Some(true),
        diagnostics: vec![],
        duration_ms: 50,
    };

    let holes = vec![(
        "api_key".to_string(),
        "\"sk-test-1234567890abcdef1234567890abcdef\"".to_string(),
    )];

    let trace = build_anonymized_trace(
        Language::Rust,
        1,
        Some("rust.axum.handler.v1".into()),
        "let secret_token = \"xyz\";",
        &holes,
        None,
        report,
    );

    assert_eq!(trace.tier, TelemetryTier::Silver);
    assert!(!trace.anonymized_scope_signature.contains("xyz"));

    let hash1 = compute_trace_hash(&trace).expect("hash");
    let hash2 = compute_trace_hash(&trace).expect("hash");
    assert_eq!(hash1, hash2, "Content hash must be deterministic");
    assert_eq!(hash1.len(), 64, "SHA-256 hex string should be 64 chars");
}

#[tokio::test]
async fn test_file_spool_sink_lifecycle() {
    let dir = tempdir().expect("tempdir");
    let spool = FileSpoolSink::new(dir.path()).await.expect("create spool");

    let report = VerificationReport {
        ast_valid: true,
        typecheck_passed: true,
        tests_passed: None,
        diagnostics: vec![],
        duration_ms: 100,
    };

    let trace = build_anonymized_trace(
        Language::Rust,
        1,
        Some("rust.test.v1".into()),
        "fn main() {}",
        &[("h1".into(), "123".into())],
        None,
        report,
    );

    // 1. Record trace
    let entry = spool.record_trace(&trace).await.expect("record trace");
    assert_eq!(entry.sync_status, SyncStatus::Pending);

    // 2. List pending
    let pending = spool.list_pending().await.expect("list pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].content_hash, entry.content_hash);

    // 3. Export batch
    let batch = spool.export_batch(10).await.expect("export batch");
    assert_eq!(batch.len(), 1);

    // 4. Mark synced
    let marked = spool
        .mark_synced(std::slice::from_ref(&entry.content_hash))
        .await
        .expect("mark synced");
    assert_eq!(marked, 1);

    // 5. Verify no pending remaining
    let remaining = spool.list_pending().await.expect("list pending");
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn test_memory_sink_lifecycle() {
    let sink = MemoryTelemetrySink::new();
    let report = VerificationReport {
        ast_valid: true,
        typecheck_passed: true,
        tests_passed: Some(true),
        diagnostics: vec![],
        duration_ms: 10,
    };
    let trace = build_anonymized_trace(
        Language::Rust,
        2,
        None,
        "let x = 1;",
        &[],
        Some("let x = false;".into()),
        report,
    );

    let entry = sink.record_trace(&trace).await.expect("record");
    assert_eq!(entry.trace.tier, TelemetryTier::Gold);
    assert!(entry.trace.rejected_attempt.is_some());

    let pending = sink.list_pending().await.expect("list");
    assert_eq!(pending.len(), 1);

    sink.mark_synced(&[entry.content_hash]).await.expect("mark");
    let after = sink.list_pending().await.expect("list");
    assert!(after.is_empty());
}

#![allow(clippy::expect_used, clippy::unwrap_used)]

use dado_core::PatternRegistry;
use joiner_core::model::{ExecutionMode, JoinerConfig, SynthesisStrategy, TelemetryTier};
use joiner_core::router::System1Router;
use miter_engine::Language;
use std::sync::Arc;
use storypole_core::model::task::{TaskKind, TaskNode, TaskScope, TaskStatus};
use uuid::Uuid;

fn make_test_task(title: &str, slug: &str, labels: Vec<String>, symbols: Vec<String>) -> TaskNode {
    TaskNode {
        id: Uuid::new_v4(),
        ticket_key: "GB-10".to_string(),
        slug: slug.to_string(),
        title: title.to_string(),
        description: "Test task".to_string(),
        kind: TaskKind::Feature,
        status: TaskStatus::Ready,
        priority: 1,
        scope: TaskScope {
            repo: Some("gaugeblock".into()),
            crate_name: Some("test-crate".into()),
            module_path: None,
            target_files: vec!["src/lib.rs".into()],
            target_symbols: symbols,
            labels,
        },
        edges: vec![],
        decisions: vec![],
        acceptance_criteria: vec![],
        active_claim: None,
        latest_handoff: None,
        verification_receipt: None,
        plan: None,
        created_at_epoch_ms: 1000,
        updated_at_epoch_ms: 1000,
    }
}

#[test]
fn test_config_defaults_and_serialization() {
    let cfg = JoinerConfig::default();
    assert_eq!(cfg.execution_mode, ExecutionMode::InPlace);
    assert!(cfg.parallel_holes);
    assert_eq!(cfg.max_repair_attempts, 3);

    let json = serde_json::to_string(&cfg).expect("serialize config");
    let deserialized: JoinerConfig = serde_json::from_str(&json).expect("deserialize config");
    assert_eq!(cfg, deserialized);
}

#[tokio::test]
async fn test_router_deterministic_label_match() {
    let mut registry = PatternRegistry::new().expect("load embedded patterns");
    registry.load_embedded().ok();
    let reg_arc = Arc::new(registry);
    let router = System1Router::new(reg_arc.clone(), None);

    // rust.axum.handler.sqlx_tx is one of the embedded patterns in dado
    let pats = reg_arc.list_patterns(Some(Language::Rust), None);
    assert!(!pats.is_empty(), "Embedded patterns should be present");
    let sample_id = pats[0].pattern_id.clone();

    let task = make_test_task(
        "Implement endpoint",
        "implement-endpoint",
        vec![format!("pattern:{sample_id}")],
        vec![],
    );

    let strategy = router.route(&task, Language::Rust).await.expect("route");
    match strategy {
        SynthesisStrategy::PatternMatch { pattern_id, mixins } => {
            assert_eq!(pattern_id, sample_id);
            assert!(mixins.is_empty());
        }
        other => panic!("Expected PatternMatch, got {other:?}"),
    }
}

#[tokio::test]
async fn test_router_surgical_and_oracle_fallbacks() {
    let registry = Arc::new(PatternRegistry::new().expect("load patterns"));
    let router = System1Router::new(registry, None);

    // Surgical edit fallback when symbols are present
    let task_surgical = make_test_task(
        "Refactor function signature",
        "refactor-sig",
        vec!["refactor".into()],
        vec!["crate::engine::execute".into()],
    );

    let strategy = router
        .route(&task_surgical, Language::Rust)
        .await
        .expect("route");
    match strategy {
        SynthesisStrategy::SurgicalEdit { target_symbol, .. } => {
            assert_eq!(target_symbol, "crate::engine::execute");
        }
        other => panic!("Expected SurgicalEdit, got {other:?}"),
    }

    // Oracle fallback when no patterns match and no symbols
    let task_oracle = make_test_task(
        "Novel neural algorithm implementation",
        "novel-algo",
        vec!["research".into()],
        vec![],
    );

    let strategy_oracle = router
        .route(&task_oracle, Language::Rust)
        .await
        .expect("route");
    match strategy_oracle {
        SynthesisStrategy::OracleFallback { rationale } => {
            assert!(rationale.contains("Novel neural algorithm"));
        }
        other => panic!("Expected OracleFallback, got {other:?}"),
    }
}

#[test]
fn test_telemetry_tier_roundtrip() {
    let tiers = vec![
        TelemetryTier::Bronze,
        TelemetryTier::Silver,
        TelemetryTier::Gold,
        TelemetryTier::Diamond,
    ];
    for tier in tiers {
        let json = serde_json::to_string(&tier).expect("serialize tier");
        let de: TelemetryTier = serde_json::from_str(&json).expect("deserialize tier");
        assert_eq!(tier, de);
    }
}

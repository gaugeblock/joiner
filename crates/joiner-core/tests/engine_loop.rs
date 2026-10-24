#![allow(clippy::expect_used, clippy::unwrap_used)]

use dado_core::PatternRegistry;
use joiner_core::JoinerEngine;
use joiner_core::model::{JoinerConfig, SynthesisEvent, SynthesisStrategy};
use joiner_core::telemetry::{MemoryTelemetrySink, TelemetrySink};
use sawhorse_core::config::{ProviderConfig, RoleConfig, SawhorseConfig};
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::{ExecutionMode, ModelRole};
use std::collections::HashMap;
use std::sync::Arc;
use storypole_core::model::task::{TaskKind, TaskNode, TaskScope, TaskStatus};
use uuid::Uuid;

fn build_test_router() -> Arc<RoleRouter> {
    let mut providers = HashMap::new();
    providers.insert(
        "test-provider".to_string(),
        ProviderConfig {
            mode: ExecutionMode::OpenAiCompatible {
                base_url: "http://127.0.0.1:9999/v1".into(),
                api_key_env: "NONE".into(),
                model_name: "mock".into(),
            },
            dialect: None,
            headers: None,
            timeout_ms: None,
        },
    );

    let mut roles = HashMap::new();
    for role in [
        ModelRole::System1Router,
        ModelRole::HoleFiller,
        ModelRole::Oracle,
        ModelRole::Planner,
    ] {
        roles.insert(
            role,
            RoleConfig {
                provider: "test-provider".to_string(),
                fallback: vec![],
            },
        );
    }

    let config = SawhorseConfig {
        providers,
        roles,
        timeouts: Default::default(),
    };

    Arc::new(RoleRouter::from_config(config).expect("create router"))
}

#[tokio::test]
async fn test_engine_event_broadcast_and_synthesis() {
    let reg = Arc::new(PatternRegistry::new().expect("patterns"));
    let router = build_test_router();
    let sink = Arc::new(MemoryTelemetrySink::new());
    let config = JoinerConfig::default();

    let engine = JoinerEngine::new(config, reg, router, sink.clone());
    let mut rx = engine.subscribe();

    let task = TaskNode {
        id: Uuid::new_v4(),
        ticket_key: "GB-99".into(),
        slug: "test-feature".into(),
        title: "Surgical Refactor".into(),
        description: "Perform surgical refactor".into(),
        kind: TaskKind::Refactor,
        status: TaskStatus::Ready,
        priority: 1,
        scope: TaskScope {
            repo: Some("test".into()),
            crate_name: Some("test-crate".into()),
            module_path: None,
            target_files: vec!["src/lib.rs".into()],
            target_symbols: vec!["crate::sym".into()],
            labels: vec![],
        },
        edges: vec![],
        decisions: vec![],
        acceptance_criteria: vec![],
        active_claim: None,
        latest_handoff: None,
        verification_receipt: None,
        plan: None,
        created_at_epoch_ms: 100,
        updated_at_epoch_ms: 100,
    };

    let result = engine.synthesize_task(&task).await.expect("synthesize");
    assert_eq!(result.ticket_key, "GB-99");
    assert!(matches!(result.strategy, SynthesisStrategy::SurgicalEdit { .. }));

    // Verify events were emitted
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SynthesisEvent::StrategySelected { .. })),
        "StrategySelected event should be emitted"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SynthesisEvent::Verified { .. })),
        "Verified event should be emitted"
    );

    // Verify telemetry trace was recorded
    let pending = sink.list_pending().await.expect("pending");
    assert_eq!(pending.len(), 1);
}

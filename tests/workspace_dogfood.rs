#![allow(clippy::expect_used, clippy::unwrap_used)]

use dado_core::PatternRegistry;
use joiner_core::JoinerEngine;
use joiner_core::model::JoinerConfig;
use joiner_core::telemetry::{MemoryTelemetrySink, TelemetrySink};
use sawhorse_core::config::{ProviderConfig, RoleConfig, SawhorseConfig};
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::{ExecutionMode, ModelRole};
use std::collections::HashMap;
use std::sync::Arc;
use storypole_core::config::StorypoleConfig;
use storypole_core::model::task::{TaskKind, TaskScope, TaskStatus};
use storypole_core::ops::mutations::{CreateTaskParams, add_task};
use storypole_core::store::StorypoleStore;
use tempfile::tempdir;

fn build_test_router() -> Arc<RoleRouter> {
    let mut providers = HashMap::new();
    providers.insert(
        "mock-provider".to_string(),
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
                provider: "mock-provider".to_string(),
                fallback: vec![],
            },
        );
    }

    let config = SawhorseConfig {
        providers,
        roles,
        timeouts: Default::default(),
    };

    Arc::new(RoleRouter::from_config(config).expect("router"))
}

#[tokio::test]
async fn test_full_workspace_synthesis_and_storypole_verification() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path();

    // 1. Initialize storypole repository
    let sp_config = StorypoleConfig {
        ticket_prefix: "GB".into(),
        allowed_repos: vec![],
        allowed_crates: vec![],
        claim_ttl_minutes: 10,
        verification_commands: vec![],
    };
    let mut store = StorypoleStore::init(root, sp_config).expect("init storypole");

    // 2. Add ready task
    let params = CreateTaskParams {
        title: "Surgical Refactor".into(),
        description: "Perform surgical code update".into(),
        kind: TaskKind::Refactor,
        priority: 1,
        scope: TaskScope {
            repo: Some("gaugeblock".into()),
            crate_name: Some("test-crate".into()),
            module_path: None,
            target_files: vec!["src/lib.rs".into()],
            target_symbols: vec!["crate::sym".into()],
            labels: vec![],
        },
        acceptance_criteria: vec![],
        status: Some(TaskStatus::Ready),
        edges: vec![],
    };
    let created_task = add_task(&mut store, params).expect("add task");
    assert_eq!(created_task.status, TaskStatus::Ready);

    // 3. Initialize JoinerEngine
    let config = JoinerConfig {
        workspace_root: root.to_path_buf(),
        ..Default::default()
    };
    let reg = Arc::new(PatternRegistry::new().expect("patterns"));
    let router = build_test_router();
    let sink = Arc::new(MemoryTelemetrySink::new());

    let engine = JoinerEngine::new(config, reg, router, sink.clone());

    // 4. Execute run-next
    let result = engine.run_next().await.expect("run_next");
    assert_eq!(result.ticket_key, "GB-1");

    // 5. Verify storypole task state transitioned to Verified
    let reloaded_store = StorypoleStore::open(root).expect("reopen");
    let all_tasks =
        storypole_core::index::query::get_all_tasks(reloaded_store.conn()).expect("get tasks");
    let verified_task = all_tasks
        .values()
        .find(|t| t.ticket_key == "GB-1")
        .expect("task found");
    assert_eq!(verified_task.status, TaskStatus::Verified);
    assert!(verified_task.verification_receipt.is_some());

    // 6. Verify telemetry record was saved in sink
    let pending = sink.list_pending().await.expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].trace.selected_pattern_id, None);
}

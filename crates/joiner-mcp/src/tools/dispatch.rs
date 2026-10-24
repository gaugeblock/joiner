//! MCP tool execution handlers for `joiner`.

use super::build_default_router;
use joiner_core::JoinerEngine;
use joiner_core::model::{ExecutionMode, JoinerConfig};
use joiner_core::telemetry::{FileSpoolSink, TelemetrySink};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use storypole_core::index::query::get_all_tasks;
use storypole_core::store::StorypoleStore;

/// Dispatches an incoming MCP tool invocation.
pub async fn dispatch_tool(name: &str, args: &Value) -> Value {
    match name {
        "joiner_run_next" => handle_run_next(args).await,
        "joiner_synthesize" => handle_synthesize(args).await,
        "joiner_status" => handle_status(args).await,
        other => json!({
            "isError": true,
            "content": [{
                "type": "text",
                "text": format!("Unknown tool: {other}")
            }]
        }),
    }
}

async fn handle_run_next(args: &Value) -> Value {
    let root = args
        .get("workspace_root")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let mode = args
        .get("execution_mode")
        .and_then(|v| v.as_str())
        .map(|m| match m {
            "sandbox" => ExecutionMode::Sandbox,
            _ => ExecutionMode::InPlace,
        })
        .unwrap_or(ExecutionMode::InPlace);

    let config = JoinerConfig {
        workspace_root: root,
        execution_mode: mode,
        ..Default::default()
    };

    let reg = Arc::new(dado_core::PatternRegistry::new().unwrap_or_default());
    let router = build_default_router();
    let sink = match FileSpoolSink::new(&config.spool_dir).await {
        Ok(s) => Arc::new(s) as Arc<dyn TelemetrySink>,
        Err(_) => Arc::new(joiner_core::telemetry::MemoryTelemetrySink::new()),
    };

    let engine = JoinerEngine::new(config, reg, router, sink);
    match engine.run_next().await {
        Ok(res) => json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&res).unwrap_or_default()
            }]
        }),
        Err(err) => json!({
            "isError": true,
            "content": [{
                "type": "text",
                "text": format!("joiner run-next failed: {err}")
            }]
        }),
    }
}

async fn handle_synthesize(args: &Value) -> Value {
    let task_id_str = match args.get("task_id").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return json!({
                "isError": true,
                "content": [{ "type": "text", "text": "Missing required argument 'task_id'" }]
            });
        }
    };

    let root = args
        .get("workspace_root")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let store = match StorypoleStore::open(&root) {
        Ok(s) => s,
        Err(e) => {
            return json!({
                "isError": true,
                "content": [{ "type": "text", "text": format!("Failed opening storypole: {e}") }]
            });
        }
    };

    let all_tasks = match get_all_tasks(store.conn()) {
        Ok(t) => t,
        Err(e) => {
            return json!({
                "isError": true,
                "content": [{ "type": "text", "text": format!("Failed querying tasks: {e}") }]
            });
        }
    };

    let target_task = all_tasks.values().find(|t| {
        t.ticket_key.eq_ignore_ascii_case(task_id_str)
            || t.id.to_string() == task_id_str
    });

    let task = match target_task {
        Some(t) => t.clone(),
        None => {
            return json!({
                "isError": true,
                "content": [{ "type": "text", "text": format!("Task not found: {task_id_str}") }]
            });
        }
    };

    let config = JoinerConfig {
        workspace_root: root,
        ..Default::default()
    };
    let reg = Arc::new(dado_core::PatternRegistry::new().unwrap_or_default());
    let router = build_default_router();
    let sink = Arc::new(joiner_core::telemetry::MemoryTelemetrySink::new());
    let engine = JoinerEngine::new(config, reg, router, sink);

    match engine.synthesize_task(&task).await {
        Ok(res) => json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&res).unwrap_or_default()
            }]
        }),
        Err(err) => json!({
            "isError": true,
            "content": [{
                "type": "text",
                "text": format!("joiner synthesize failed: {err}")
            }]
        }),
    }
}

async fn handle_status(args: &Value) -> Value {
    let root = args
        .get("workspace_root")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let storypole_status = match StorypoleStore::open(&root) {
        Ok(store) => match get_all_tasks(store.conn()) {
            Ok(tasks) => {
                let ready = tasks.values().filter(|t| t.status == storypole_core::model::task::TaskStatus::Ready).count();
                let in_progress = tasks.values().filter(|t| t.status == storypole_core::model::task::TaskStatus::InProgress).count();
                let verified = tasks.values().filter(|t| t.status == storypole_core::model::task::TaskStatus::Verified).count();
                json!({ "ready": ready, "in_progress": in_progress, "verified": verified, "total": tasks.len() })
            }
            Err(e) => json!({ "error": e.to_string() }),
        },
        Err(e) => json!({ "uninitialized": e.to_string() }),
    };

    let spool_dir = joiner_core::model::JoinerConfig::default_spool_dir(&root);
    let spool_sink = FileSpoolSink::new(&spool_dir).await;
    let pending_traces = match spool_sink {
        Ok(s) => s.list_pending().await.map(|v| v.len()).unwrap_or(0),
        Err(_) => 0,
    };

    json!({
        "content": [{
            "type": "text",
            "text": json!({
                "service": "joiner-mcp",
                "version": env!("CARGO_PKG_VERSION"),
                "pending_telemetry_traces": pending_traces,
                "storypole": storypole_status
            }).to_string()
        }]
    })
}


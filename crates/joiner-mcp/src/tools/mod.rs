//! MCP tool schema definitions and execution dispatch for `joiner`.

pub mod dispatch;

pub use dispatch::dispatch_tool;
use serde_json::{Value, json};
use std::sync::Arc;

/// Returns JSON schemas for all tools exposed by `joiner-mcp`.
#[must_use]
pub fn list_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "joiner_run_next",
            "description": "Claims the next ready task from storypole, executes System 1 routing, fills typed holes, runs compiler/test verification, records telemetry, and marks the task Verified.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workspace_root": {
                        "type": "string",
                        "description": "Path to workspace repository root (defaults to '.')"
                    },
                    "execution_mode": {
                        "type": "string",
                        "enum": ["in_place", "sandbox"],
                        "description": "Execution isolation mode (in_place by default)"
                    }
                }
            }
        }),
        json!({
            "name": "joiner_synthesize",
            "description": "Synthesizes and verifies code for a specific storypole task by ticket key or UUID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task UUID or ticket key (e.g. 'GB-5')"
                    },
                    "workspace_root": {
                        "type": "string",
                        "description": "Path to workspace repository root (defaults to '.')"
                    },
                    "execution_mode": {
                        "type": "string",
                        "enum": ["in_place", "sandbox"],
                        "description": "Execution isolation mode"
                    }
                },
                "required": ["task_id"]
            }
        }),
        json!({
            "name": "joiner_status",
            "description": "Returns current joiner runner status, pending telemetry spool count, and storypole ready tasks.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workspace_root": {
                        "type": "string",
                        "description": "Path to workspace root (defaults to '.')"
                    }
                }
            }
        }),
    ]
}

pub(crate) fn build_default_router() -> Arc<sawhorse_core::router::RoleRouter> {
    let cfg = sawhorse_core::config::SawhorseConfig {
        providers: Default::default(),
        roles: Default::default(),
        timeouts: Default::default(),
    };
    Arc::new(
        sawhorse_core::router::RoleRouter::from_config(cfg)
            .unwrap_or_else(|_| create_fallback_router()),
    )
}

fn create_fallback_router() -> sawhorse_core::router::RoleRouter {
    match sawhorse_core::router::RoleRouter::from_config(sawhorse_core::config::SawhorseConfig {
        providers: Default::default(),
        roles: Default::default(),
        timeouts: Default::default(),
    }) {
        Ok(r) => r,
        Err(_) => unreachable!("Empty SawhorseConfig is infallible"),
    }
}


//! Stdio JSON-RPC 2.0 Model Context Protocol server loop.

use crate::tools::{dispatch_tool, list_tools};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use tracing::{error, info};

/// Runs the blocking stdio JSON-RPC 2.0 loop reading lines from stdin.
pub async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    info!("Starting joiner-mcp stdio server loop");

    for line_res in stdin.lock().lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(e) => {
                error!("Failed reading stdin line: {e}");
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed parsing JSON-RPC: {e}");
                continue;
            }
        };

        if let Some(resp) = handle_message(&msg).await {
            let resp_str = serde_json::to_string(&resp)?;
            writeln!(stdout, "{resp_str}")?;
            stdout.flush()?;
        }
    }

    Ok(())
}

/// Processes a single JSON-RPC 2.0 request and returns the response Value.
pub async fn handle_message(msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(|v| v.as_str())?;

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "joiner-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        })),
        "notifications/initialized" => None,
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": list_tools()
            }
        })),
        "tools/call" => {
            let params = msg.get("params");
            let tool_name = params
                .and_then(|p| p.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = params
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or_else(|| json!({}));

            let result = dispatch_tool(tool_name, &arguments).await;
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            }))
        }
        _ => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {method}")
            }
        })),
    }
}


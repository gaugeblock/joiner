//! Stdio JSON-RPC 2.0 Model Context Protocol server for `joiner`.

use crate::tools;
use gaugeblock_mcp::{AsyncMcpHandler, RpcRequest, RpcResponse, ToolOutcome};
use serde_json::Value;
use std::io::{self};

/// `joiner`'s tool surface.
#[derive(Debug, Default)]
pub struct JoinerHandler;

impl AsyncMcpHandler for JoinerHandler {
    fn server_name(&self) -> &str {
        "joiner-mcp"
    }

    fn list_tools(&self) -> Value {
        Value::Array(tools::list_tools())
    }

    async fn call_tool(&self, name: &str, arguments: &Value) -> ToolOutcome {
        match tools::dispatch_tool(name, arguments).await {
            // The dispatcher formats its reports; the transport wraps them.
            Ok(text) => ToolOutcome::Text(text),
            Err(message) => ToolOutcome::Failed(message),
        }
    }
}

/// Runs the blocking stdio JSON-RPC 2.0 loop reading lines from stdin.
///
/// # Errors
///
/// Returns the I/O failure when stdin cannot be read or stdout cannot be written.
pub async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Starting joiner-mcp stdio server loop");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    gaugeblock_mcp::serve_async(stdin.lock(), &mut stdout, &JoinerHandler)
        .await
        .map_err(Box::<dyn std::error::Error + Send + Sync>::from)?;
    Ok(())
}

/// Processes a single JSON-RPC 2.0 request and returns the response Value.
///
/// Kept because the existing tests drive the server this way; the routing lives in `gaugeblock-mcp`.
#[must_use]
pub async fn handle_message(msg: &Value) -> Option<Value> {
    let request: RpcRequest = serde_json::from_value(msg.clone()).ok()?;
    gaugeblock_mcp::dispatch_async(&JoinerHandler, &request)
        .await
        .map(RpcResponse::into_value)
}

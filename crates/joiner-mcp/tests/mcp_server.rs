#![allow(clippy::expect_used, clippy::unwrap_used)]

use joiner_mcp::{handle_message, list_tools};
use serde_json::json;

#[tokio::test]
async fn test_mcp_initialize() {
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });

    let resp = handle_message(&req).await.expect("initialize response");
    assert_eq!(resp["result"]["serverInfo"]["name"], "joiner-mcp");
    assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
}

#[tokio::test]
async fn test_mcp_tools_list() {
    let tools = list_tools();
    assert_eq!(tools.len(), 3);
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"joiner_run_next"));
    assert!(names.contains(&"joiner_synthesize"));
    assert!(names.contains(&"joiner_status"));

    let req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });
    let resp = handle_message(&req).await.expect("tools/list response");
    assert!(resp["result"]["tools"].as_array().is_some());
}

#[tokio::test]
async fn test_mcp_tool_call_status() {
    let req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "joiner_status",
            "arguments": {
                "workspace_root": "."
            }
        }
    });

    let resp = handle_message(&req).await.expect("call status");
    assert!(resp["result"]["content"].as_array().is_some());
}

#[tokio::test]
async fn test_mcp_unknown_method() {
    let req = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "non_existent_method"
    });

    let resp = handle_message(&req).await.expect("error response");
    assert_eq!(resp["error"]["code"], -32601);
}


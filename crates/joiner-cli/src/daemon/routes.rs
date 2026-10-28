//! Axum REST endpoints, WebSocket upgrade loop, and SSE streaming handlers.

use super::AppState;
use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::Stream;
use joiner_core::model::SynthesisResult;
use joiner_core::telemetry::SpoolEntry;
use serde_json::{Value, json};
use std::convert::Infallible;
use std::time::Duration;
use storypole_core::index::query::get_all_tasks;
use storypole_core::store::StorypoleStore;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{error, info};

/// Health and status check endpoint.
pub async fn get_status(State(state): State<AppState>) -> Json<Value> {
    let tasks_stat = match StorypoleStore::open(&state.workspace_root) {
        Ok(store) => match get_all_tasks(store.conn()) {
            Ok(tasks) => {
                let ready = tasks
                    .values()
                    .filter(|t| t.status == storypole_core::model::task::TaskStatus::Ready)
                    .count();
                let in_progress = tasks
                    .values()
                    .filter(|t| t.status == storypole_core::model::task::TaskStatus::InProgress)
                    .count();
                let verified = tasks
                    .values()
                    .filter(|t| t.status == storypole_core::model::task::TaskStatus::Verified)
                    .count();
                json!({ "ready": ready, "in_progress": in_progress, "verified": verified, "total": tasks.len() })
            }
            Err(e) => json!({ "error": e.to_string() }),
        },
        Err(e) => json!({ "uninitialized": e.to_string() }),
    };

    let pending = state.engine.config().spool_dir.join("outbox.jsonl");

    Json(json!({
        "service": "joiner-daemon",
        "version": env!("CARGO_PKG_VERSION"),
        "workspace_root": state.workspace_root.display().to_string(),
        "spool_outbox_exists": pending.exists(),
        "storypole": tasks_stat
    }))
}

/// Triggers synthesis of the next ready task.
pub async fn post_run_next(
    State(state): State<AppState>,
) -> Result<Json<SynthesisResult>, StatusCode> {
    match state.engine.run_next().await {
        Ok(res) => Ok(Json(res)),
        Err(err) => {
            error!("run-next synthesis failed: {err}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Triggers synthesis of a specific task.
pub async fn post_synthesize(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<SynthesisResult>, StatusCode> {
    let store = StorypoleStore::open(&state.workspace_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tasks = get_all_tasks(store.conn()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let task = tasks
        .values()
        .find(|t| t.ticket_key.eq_ignore_ascii_case(&task_id) || t.id.to_string() == task_id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    match state.engine.synthesize_task(&task).await {
        Ok(res) => Ok(Json(res)),
        Err(err) => {
            error!("synthesize task failed: {err}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Lists pending telemetry outbox entries.
pub async fn get_pending_telemetry(
    State(state): State<AppState>,
) -> Result<Json<Vec<SpoolEntry>>, StatusCode> {
    state
        .sink
        .list_pending()
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// WebSocket upgrade handler for bidirectional event streaming.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.engine.subscribe();
    info!("Benchwork client connected to WebSocket");

    while let Ok(event) = rx.recv().await {
        if let Ok(msg_str) = serde_json::to_string(&event)
            && socket.send(Message::Text(msg_str.into())).await.is_err()
        {
            break;
        }
    }
    info!("Benchwork WebSocket client disconnected");
}

/// Server-Sent Events (SSE) stream broadcasting synthesis lifecycle events.
pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.engine.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(event) => serde_json::to_string(&event)
            .ok()
            .map(|data| Ok(Event::default().data(data))),
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

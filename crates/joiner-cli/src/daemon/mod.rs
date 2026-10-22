//! Axum server startup, application state, and router configuration for the benchwork daemon.

pub mod routes;

use axum::routing::{get, post};
use axum::Router;
use joiner_core::JoinerEngine;
use joiner_core::model::JoinerConfig;
use joiner_core::telemetry::{FileSpoolSink, TelemetrySink};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// Shared application state for all daemon handlers.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Active repository root directory.
    pub workspace_root: PathBuf,
    /// Master orchestration engine.
    pub engine: JoinerEngine,
    /// Telemetry sink for querying pending outbox records.
    pub sink: Arc<dyn TelemetrySink>,
}

/// Constructs the Axum application router.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/v1/status", get(routes::get_status))
        .route("/api/v1/run-next", post(routes::post_run_next))
        .route("/api/v1/synthesize/{task_id}", post(routes::post_synthesize))
        .route("/api/v1/telemetry/pending", get(routes::get_pending_telemetry))
        .route("/api/v1/ws", get(routes::ws_handler))
        .route("/api/v1/events", get(routes::sse_handler))
        .layer(cors)
        .with_state(state)
}

/// Launches the joiner HTTP/WebSocket daemon.
pub async fn run_daemon(
    host: &str,
    port: u16,
    workspace_root: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = JoinerConfig {
        workspace_root: workspace_root.clone(),
        ..Default::default()
    };

    let reg = Arc::new(dado_core::PatternRegistry::new().unwrap_or_default());
    let router = build_default_router();
    let sink: Arc<dyn TelemetrySink> = match FileSpoolSink::new(&config.spool_dir).await {
        Ok(s) => Arc::new(s),
        Err(_) => Arc::new(joiner_core::telemetry::MemoryTelemetrySink::new()),
    };

    let engine = JoinerEngine::new(config, reg, router, sink.clone());
    let state = AppState {
        workspace_root,
        engine,
        sink,
    };

    let app = build_router(state);
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    info!("Joiner daemon listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn build_default_router() -> Arc<sawhorse_core::router::RoleRouter> {
    let cfg = sawhorse_core::config::SawhorseConfig {
        providers: Default::default(),
        roles: Default::default(),
        timeouts: Default::default(),
    };
    Arc::new(sawhorse_core::router::RoleRouter::from_config(cfg).unwrap_or_else(|_| {
        match sawhorse_core::router::RoleRouter::from_config(sawhorse_core::config::SawhorseConfig {
            providers: Default::default(),
            roles: Default::default(),
            timeouts: Default::default(),
        }) {
            Ok(r) => r,
            Err(_) => unreachable!("Empty SawhorseConfig is infallible"),
        }
    }))
}


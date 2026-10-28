#![allow(clippy::expect_used, clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use clap::Parser;
use joiner_cli::args::{CliArgs, Command};
use joiner_cli::daemon::{AppState, build_router};
use joiner_core::JoinerEngine;
use joiner_core::model::JoinerConfig;
use joiner_core::telemetry::MemoryTelemetrySink;
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceExt;

#[test]
fn test_cli_parsing_run_next() {
    let args = CliArgs::parse_from(["joiner", "run-next", "--sandbox", "--sequential", "--json"]);
    match args.command {
        Command::RunNext(cmd) => {
            assert!(cmd.sandbox);
            assert!(cmd.sequential);
            assert!(cmd.json);
            assert_eq!(cmd.dir, PathBuf::from("."));
        }
        _ => panic!("Expected RunNext command"),
    }
}

#[test]
fn test_cli_parsing_synthesize() {
    let args = CliArgs::parse_from(["joiner", "synthesize", "GB-5", "--dir", "my-repo"]);
    match args.command {
        Command::Synthesize(cmd) => {
            assert_eq!(cmd.task_id, "GB-5");
            assert_eq!(cmd.dir, PathBuf::from("my-repo"));
        }
        _ => panic!("Expected Synthesize command"),
    }
}

#[test]
fn test_cli_parsing_daemon() {
    let args = CliArgs::parse_from(["joiner", "daemon", "--port", "8080", "--host", "0.0.0.0"]);
    match args.command {
        Command::Daemon(cmd) => {
            assert_eq!(cmd.port, 8080);
            assert_eq!(cmd.host, "0.0.0.0");
        }
        _ => panic!("Expected Daemon command"),
    }
}

#[test]
fn test_cli_parsing_telemetry() {
    let args = CliArgs::parse_from(["joiner", "telemetry", "--export", "--limit", "50"]);
    match args.command {
        Command::Telemetry(cmd) => {
            assert!(cmd.export);
            assert_eq!(cmd.limit, 50);
        }
        _ => panic!("Expected Telemetry command"),
    }
}

#[tokio::test]
async fn test_daemon_status_endpoint() {
    let config = JoinerConfig::default();
    let reg = Arc::new(dado_core::PatternRegistry::new().unwrap_or_default());
    let sawhorse_cfg = sawhorse_core::config::SawhorseConfig {
        providers: Default::default(),
        roles: Default::default(),
        timeouts: Default::default(),
    };
    let router = Arc::new(sawhorse_core::router::RoleRouter::from_config(sawhorse_cfg).unwrap());
    let sink = Arc::new(MemoryTelemetrySink::new());
    let engine = JoinerEngine::new(config, reg, router, sink.clone());

    let state = AppState {
        workspace_root: PathBuf::from("."),
        engine,
        sink,
    };

    let app = build_router(state);
    let req = Request::builder()
        .uri("/api/v1/status")
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("oneshot call");
    assert_eq!(resp.status(), StatusCode::OK);
}

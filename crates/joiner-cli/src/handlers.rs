//! CLI command handlers for `joiner`.

use crate::args::{DaemonArgs, RunNextArgs, SynthesizeArgs, TelemetryArgs};
use crate::daemon::run_daemon;
use joiner_core::JoinerEngine;
use joiner_core::model::{ExecutionMode, JoinerConfig};
use joiner_core::telemetry::{FileSpoolSink, TelemetrySink};
use std::path::Path;
use std::sync::Arc;
use storypole_core::index::query::get_all_tasks;
use storypole_core::store::StorypoleStore;
use tracing::info;

/// Executes `joiner run-next`.
pub async fn handle_run_next(
    args: RunNextArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mode = if args.sandbox {
        ExecutionMode::Sandbox
    } else {
        ExecutionMode::InPlace
    };

    let config = JoinerConfig {
        workspace_root: args.dir,
        execution_mode: mode,
        parallel_holes: !args.sequential,
        ..Default::default()
    };

    let reg = Arc::new(dado_core::PatternRegistry::new().unwrap_or_default());
    let router = build_default_router();
    let sink: Arc<dyn TelemetrySink> = match FileSpoolSink::new(&config.spool_dir).await {
        Ok(s) => Arc::new(s),
        Err(_) => Arc::new(joiner_core::telemetry::MemoryTelemetrySink::new()),
    };

    let engine = JoinerEngine::new(config, reg, router, sink);
    let result = engine.run_next().await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Task {} successfully verified!", result.ticket_key);
        println!("Strategy: {:?}", result.strategy);
        println!("Tier:     {:?}", result.tier);
        println!("Attempts: {}", result.attempts);
    }

    Ok(())
}

/// Executes `joiner synthesize <task_id>`.
pub async fn handle_synthesize(
    args: SynthesizeArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mode = if args.sandbox {
        ExecutionMode::Sandbox
    } else {
        ExecutionMode::InPlace
    };

    let config = JoinerConfig {
        workspace_root: args.dir.clone(),
        execution_mode: mode,
        ..Default::default()
    };

    let store = StorypoleStore::open(&args.dir)?;
    let all_tasks = get_all_tasks(store.conn())?;
    let task = all_tasks
        .values()
        .find(|t| {
            t.ticket_key.eq_ignore_ascii_case(&args.task_id) || t.id.to_string() == args.task_id
        })
        .cloned()
        .ok_or_else(|| format!("Task '{}' not found", args.task_id))?;

    let reg = Arc::new(dado_core::PatternRegistry::new().unwrap_or_default());
    let router = build_default_router();
    let sink = Arc::new(joiner_core::telemetry::MemoryTelemetrySink::new());

    let engine = JoinerEngine::new(config, reg, router, sink);
    let result = engine.synthesize_task(&task).await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Task {} synthesized successfully!", result.ticket_key);
        println!("Strategy: {:?}", result.strategy);
        println!("Tier:     {:?}", result.tier);
    }

    Ok(())
}

/// Executes `joiner daemon`.
pub async fn handle_daemon(
    args: DaemonArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting joiner daemon on {}:{}", args.host, args.port);
    run_daemon(&args.host, args.port, args.dir).await
}

/// Executes `joiner mcp`.
pub async fn handle_mcp() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    joiner_mcp::run_stdio_server().await
}

/// Executes `joiner telemetry`.
pub async fn handle_telemetry(
    args: TelemetryArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let spool_dir = args
        .spool_dir
        .unwrap_or_else(|| JoinerConfig::default_spool_dir(Path::new(".")));
    let sink = FileSpoolSink::new(&spool_dir).await?;
    if args.export {
        let entries = sink.export_batch(args.limit).await?;
        for entry in entries {
            println!("{}", serde_json::to_string(&entry)?);
        }
    } else {
        let pending = sink.list_pending().await?;
        println!("Telemetry Spool Outbox: {}", spool_dir.display());
        println!("Pending traces: {}", pending.len());
    }
    Ok(())
}

fn build_default_router() -> Arc<sawhorse_core::router::RoleRouter> {
    let cfg = sawhorse_core::config::SawhorseConfig {
        providers: Default::default(),
        roles: Default::default(),
        timeouts: Default::default(),
    };
    Arc::new(
        sawhorse_core::router::RoleRouter::from_config(cfg).unwrap_or_else(|_| {
            match sawhorse_core::router::RoleRouter::from_config(
                sawhorse_core::config::SawhorseConfig {
                    providers: Default::default(),
                    roles: Default::default(),
                    timeouts: Default::default(),
                },
            ) {
                Ok(r) => r,
                Err(_) => unreachable!("Empty SawhorseConfig is infallible"),
            }
        }),
    )
}

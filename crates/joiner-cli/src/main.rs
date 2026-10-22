//! Command-line entrypoint for `joiner`.

use clap::Parser;
use joiner_cli::args;
use joiner_cli::handlers;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = args::CliArgs::parse();

    // Direct all structured logs to stderr so stdout remains JSON-clean
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    match args.command {
        args::Command::RunNext(cmd_args) => handlers::handle_run_next(cmd_args).await?,
        args::Command::Synthesize(cmd_args) => handlers::handle_synthesize(cmd_args).await?,
        args::Command::Daemon(cmd_args) => handlers::handle_daemon(cmd_args).await?,
        args::Command::Mcp => handlers::handle_mcp().await?,
        args::Command::Telemetry(cmd_args) => handlers::handle_telemetry(cmd_args).await?,
    }

    Ok(())
}

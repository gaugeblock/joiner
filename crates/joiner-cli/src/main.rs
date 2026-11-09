//! Command-line entrypoint for `joiner`.

use clap::Parser;
use joiner_cli::args;
use joiner_cli::handlers;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // One span per invocation, so the panel can answer what the suite did rather than only what inference
    // cost (`GB-339`). Opened before any work and closed with the outcome; it cannot fail the command.
    let invocation = gaugeblock_workspace::InvocationSpan::start("joiner", None);

    let args = args::CliArgs::parse();

    // Direct all structured logs to stderr so stdout remains JSON-clean
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    // The dispatch sits inside an async block so its result can be inspected before the span closes. With the `?`
    // operators where they were, an error returned from `main` and the guard recorded the span as still running -
    // a status that is accurate for success and silent about failure, which is the one thing a span has to get
    // right (`GB-339`).
    let result = async {
        match args.command {
            args::Command::RunNext(cmd_args) => handlers::handle_run_next(cmd_args).await?,
            args::Command::Synthesize(cmd_args) => handlers::handle_synthesize(cmd_args).await?,
            args::Command::Fill(cmd_args) => handlers::handle_fill(cmd_args).await?,
            args::Command::Daemon(cmd_args) => handlers::handle_daemon(cmd_args).await?,
            args::Command::Mcp => handlers::handle_mcp().await?,
            args::Command::Telemetry(cmd_args) => handlers::handle_telemetry(cmd_args).await?,
        }
        Ok(())
    }
    .await;

    invocation.finish(result.is_ok());
    result
}

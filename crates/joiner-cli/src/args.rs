//! Command-line argument definitions for `joiner`.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// CLI arguments for the `joiner` binary.
#[derive(Debug, Parser)]
#[command(
    name = "joiner",
    author = "Colin Wolff",
    version,
    about = "Master neurosymbolic synthesis orchestrator for the gaugeblock suite"
)]
pub struct CliArgs {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Commands supported by the `joiner` CLI.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Claim and synthesize the next ready task from storypole.
    RunNext(RunNextArgs),

    /// Synthesize and verify code for a specific storypole task.
    Synthesize(SynthesizeArgs),

    /// Launch HTTP/WebSocket/SSE daemon for the benchwork UI.
    Daemon(DaemonArgs),

    /// Start stdio JSON-RPC 2.0 Model Context Protocol (MCP) server.
    Mcp,

    /// Inspect or export telemetry spool traces.
    Telemetry(TelemetryArgs),
}

/// Arguments for `joiner run-next`.
#[derive(Debug, Args)]
pub struct RunNextArgs {
    /// Path to workspace root containing `.storypole` (defaults to '.').
    #[arg(short, long, default_value = ".")]
    pub dir: PathBuf,

    /// Run in isolated git worktree sandbox instead of mutating working tree directly.
    #[arg(long)]
    pub sandbox: bool,

    /// Disable parallel hole filling and process holes sequentially.
    #[arg(long)]
    pub sequential: bool,

    /// Output results as formatted JSON.
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `joiner synthesize <task_id>`.
#[derive(Debug, Args)]
pub struct SynthesizeArgs {
    /// Storypole task ID or ticket key (e.g. 'GB-5').
    pub task_id: String,

    /// Path to workspace root containing `.storypole`.
    #[arg(short, long, default_value = ".")]
    pub dir: PathBuf,

    /// Run in isolated git worktree sandbox.
    #[arg(long)]
    pub sandbox: bool,

    /// Output results as formatted JSON.
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `joiner daemon`.
#[derive(Debug, Args)]
pub struct DaemonArgs {
    /// Network port to bind the daemon.
    #[arg(short, long, default_value_t = 7680)]
    pub port: u16,

    /// Network host address to bind.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Path to workspace root directory.
    #[arg(short, long, default_value = ".")]
    pub dir: PathBuf,
}

/// Arguments for `joiner telemetry`.
#[derive(Debug, Args)]
pub struct TelemetryArgs {
    /// Spool directory path (defaults to '.gaugeblock/spool' or '.rabbet/spool').
    #[arg(long)]
    pub spool_dir: Option<PathBuf>,

    /// Export pending outbox traces to stdout.
    #[arg(long)]
    pub export: bool,

    /// Maximum records to export.
    #[arg(long, default_value_t = 100)]
    pub limit: usize,
}

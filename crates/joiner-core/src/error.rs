//! Error types for `joiner-core`.

use thiserror::Error;

/// Result alias for `joiner-core` operations.
pub type Result<T> = std::result::Result<T, JoinerError>;

/// Primary error enumeration covering all orchestration phases in `joiner`.
#[derive(Debug, Error)]
pub enum JoinerError {
    /// Failure during AST indexing or symbol querying in `miter-engine`.
    #[error("miter engine error: {0}")]
    Miter(#[from] miter_engine::EngineError),

    /// Failure during isolated worktree or compiler verification in `miter-harness`.
    #[error("miter harness error: {0}")]
    Harness(#[from] miter_harness::HarnessError),

    /// Failure interacting with `storypole` task DAG, claims, or ledger.
    #[error("storypole error: {0}")]
    Storypole(#[from] storypole_core::error::StorypoleError),

    /// Failure calling LLM providers via `sawhorse`.
    #[error("sawhorse inference error: {0}")]
    Sawhorse(#[from] sawhorse_core::error::SawhorseError),

    /// Failure parsing AST templates or splicing typed holes in `dado`.
    #[error("dado error: {0}")]
    Dado(#[from] dado_core::error::DadoError),

    /// Failure generating or validating CFG grammars in `batten`.
    #[error("batten grammar error: {0}")]
    Batten(#[from] batten_core::error::BattenError),

    /// Failure parsing or compiling specs in `chalkline`.
    #[error("chalkline error: {0}")]
    Chalkline(#[from] chalkline_core::error::ChalklineError),

    /// Standard I/O error during file mutation or trace persistence.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// No tasks in `Ready` status were found in `storypole`.
    #[error("no ready tasks available")]
    NoReadyTask,

    /// No matching `DadoPattern` could be resolved for the task scope.
    #[error("pattern not found: {0}")]
    PatternNotFound(String),

    /// Failure filling a specific typed hole.
    #[error("failed filling hole '{hole_id}': {reason}")]
    HoleFillFailed {
        /// Identifier of the target hole.
        hole_id: String,
        /// Failure explanation.
        reason: String,
    },

    /// Verification failed after exhausting the retry budget.
    #[error("verification failed after {attempts} attempts: {last_error}")]
    VerificationFailed {
        /// Number of attempts executed.
        attempts: usize,
        /// Description of the terminal error.
        last_error: String,
    },

    /// Frontier model Oracle escalation failed to produce valid code.
    #[error("oracle escalation failed: {0}")]
    OracleFailed(String),

    /// Telemetry spool outbox persistence or sync failure.
    #[error("telemetry spool error: {0}")]
    SpoolError(String),

    /// Invalid configuration supplied to the engine.
    #[error("configuration error: {0}")]
    Configuration(String),
}


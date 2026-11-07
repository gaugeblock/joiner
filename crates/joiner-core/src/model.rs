//! Core domain models, configuration, and telemetry contract types for `joiner`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Execution environment mode for synthesis and verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Mutate the working tree directly in-place.
    #[default]
    InPlace,
    /// Isolate synthesis and verification in a temporary `git worktree` via `miter-harness`.
    Sandbox,
}

/// Global orchestration configuration for `JoinerEngine`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinerConfig {
    /// Absolute path to the repository workspace root.
    pub workspace_root: PathBuf,
    /// Selected execution mode (in-place vs isolated sandbox).
    pub execution_mode: ExecutionMode,
    /// Whether independent typed holes are filled concurrently.
    pub parallel_holes: bool,
    /// Maximum self-repair retry attempts before terminal failure.
    pub max_repair_attempts: usize,
    /// Directory for the local JSONL telemetry spool outbox.
    pub spool_dir: PathBuf,
    /// Agent identifier used when acquiring claims in `storypole`.
    pub agent_id: String,
}

impl JoinerConfig {
    /// Resolves default telemetry spool directory, preferring `.gaugeblock/spool` if present.
    pub fn default_spool_dir(root: &Path) -> PathBuf {
        // `spool_dir()` states the layout once (`GB-304`). The condition is unchanged - a repository
        // with a `.gaugeblock` but no spool still gets the canonical path, so the spool is created
        // where the suite expects it rather than in a legacy location.
        let paths = gaugeblock_workspace::GaugeblockPaths::at(root);
        let gb_spool = paths.spool_dir();
        if gb_spool.is_dir() || paths.root().is_dir() {
            gb_spool
        } else {
            root.join(".rabbet").join("spool")
        }
    }
}

impl Default for JoinerConfig {
    fn default() -> Self {
        Self {
            workspace_root: PathBuf::from("."),
            execution_mode: ExecutionMode::InPlace,
            parallel_holes: true,
            max_repair_attempts: 3,
            spool_dir: Self::default_spool_dir(Path::new(".")),
            agent_id: "joiner".to_string(),
        }
    }
}

/// Selected synthesis strategy determined by the System 1 Router.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SynthesisStrategy {
    /// Deterministic `dado` pattern match with optional mixins.
    PatternMatch {
        /// Unique pattern ID from the registry.
        pattern_id: String,
        /// Optional mixin identifiers applied.
        mixins: Vec<String>,
    },
    /// Surgical AST modification.
    SurgicalEdit {
        /// Target AST symbol name or file location.
        target_symbol: String,
        /// Surgical transformation directive.
        action: String,
    },
    /// Unconstrained frontier model escalation for novel algorithms.
    OracleFallback {
        /// Prompt context or rationale for fallback.
        rationale: String,
    },
}

// The telemetry record lives in `gaugeblock-contracts` (GB-174), so a trace joiner writes is the same
// Rust type storypole and rabbet read. Re-exported to keep every existing path working.
pub use gaugeblock_contracts::{
    AnonymizedSynthesisTrace, CompilerDiagnostic, TelemetryTier, VerificationReport,
};

/// Summary result of a completed task synthesis pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisResult {
    /// Unique task ID from `storypole`.
    pub task_id: Uuid,
    /// Human-friendly ticket key (e.g. `GB-5`).
    pub ticket_key: String,
    /// Resolved synthesis strategy used.
    pub strategy: SynthesisStrategy,
    /// Quality tier attained.
    pub tier: TelemetryTier,
    /// Spliced filled holes `(hole_id, expression)`.
    pub filled_holes: Vec<(String, String)>,
    /// Total attempts required (1..=3).
    pub attempts: usize,
    /// Total wall-clock execution time in milliseconds.
    pub duration_ms: u64,
    /// Verification report.
    pub verification: VerificationReport,
}

/// Real-time streaming lifecycle events emitted by the orchestrator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum SynthesisEvent {
    /// Task successfully claimed under lease.
    TaskClaimed {
        /// Task ID.
        task_id: Uuid,
        /// Human-friendly ticket key.
        ticket_key: String,
    },
    /// Router selected an execution strategy.
    StrategySelected {
        /// Task ID.
        task_id: Uuid,
        /// Chosen strategy.
        strategy: SynthesisStrategy,
    },
    /// Compiling batten grammar for a hole.
    HoleCompiling {
        /// Task ID.
        task_id: Uuid,
        /// Hole ID.
        hole_id: String,
    },
    /// Hole filled and validated.
    HoleFilled {
        /// Task ID.
        task_id: Uuid,
        /// Hole ID.
        hole_id: String,
    },
    /// Verification pass started.
    Verifying {
        /// Task ID.
        task_id: Uuid,
        /// Attempt number.
        attempt: usize,
    },
    /// Localized self-repair attempt triggered.
    RepairAttempt {
        /// Task ID.
        task_id: Uuid,
        /// Attempt number.
        attempt: usize,
        /// Target hole being repaired, if localized.
        target_hole: Option<String>,
        /// Compiler error message driving the repair.
        diagnostic: String,
    },
    /// Task verified and closed.
    Verified {
        /// Task ID.
        task_id: Uuid,
        /// Assigned quality tier.
        tier: TelemetryTier,
    },
    /// Task failed verification.
    Failed {
        /// Task ID.
        task_id: Uuid,
        /// Terminal failure reason.
        reason: String,
    },
}

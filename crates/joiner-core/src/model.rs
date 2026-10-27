//! Core domain models, configuration, and telemetry contract types for `joiner`.

use miter_engine::Language;
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
        let gb_spool = root.join(".gaugeblock").join("spool");
        if gb_spool.is_dir() || root.join(".gaugeblock").is_dir() {
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

/// Quality tier assigned to an anonymized synthesis trace per `CONTRACTS.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryTier {
    /// Compiled and typechecked cleanly on attempt #1 without tests.
    Bronze,
    /// Passed compiler checks and unit tests on attempt #1.
    Silver,
    /// Required self-repair retry or Oracle fallback (contains DPO preference pair).
    Gold,
    /// Survived unchanged in main across subsequent release commits.
    Diamond,
}

/// Located compiler diagnostic reported during verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilerDiagnostic {
    /// Relative file path.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
    /// Tool-specific error code (e.g. `E0308`).
    pub code: Option<String>,
    /// Error message string.
    pub message: String,
    /// Optional compiler suggestion.
    pub suggestion: Option<String>,
}

/// Comprehensive outcome of verifying a synthesized file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Whether the synthesized source parsed into a valid AST.
    pub ast_valid: bool,
    /// Whether static typecheck/compilation succeeded.
    pub typecheck_passed: bool,
    /// Whether cargo/project unit tests passed, if executed.
    pub tests_passed: Option<bool>,
    /// Located diagnostics emitted by the compiler or linter.
    pub diagnostics: Vec<CompilerDiagnostic>,
    /// Elapsed verification duration in milliseconds.
    pub duration_ms: u64,
}

/// Wire contract representing an AST-anonymized synthesis run per `CONTRACTS.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnonymizedSynthesisTrace {
    /// Globally unique trace identifier.
    pub trace_id: Uuid,
    /// Programming language of the synthesized code.
    #[serde(with = "dado_core::language_serde")]
    pub language: Language,
    /// Quality tier achieved by this synthesis run.
    pub tier: TelemetryTier,
    /// Selected `dado` pattern ID, if applicable.
    pub selected_pattern_id: Option<String>,
    /// Normalized anonymized lexical scope signature.
    pub anonymized_scope_signature: String,
    /// Completed hole expressions `(hole_id, anonymized_expression)`.
    pub filled_holes: Vec<(String, String)>,
    /// Rejected candidate attempt (populated for Gold DPO pairs).
    pub rejected_attempt: Option<String>,
    /// Detailed verification report.
    pub verification: VerificationReport,
}

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

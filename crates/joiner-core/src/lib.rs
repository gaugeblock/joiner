//! # `joiner-core`
//!
//! Master neurosymbolic synthesis orchestrator for the `gaugeblock` suite.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod error;
pub mod model;
pub mod router;

pub use error::{JoinerError, Result};
pub use model::{
    AnonymizedSynthesisTrace, CompilerDiagnostic, ExecutionMode, JoinerConfig,
    SynthesisEvent, SynthesisResult, SynthesisStrategy, TelemetryTier, VerificationReport,
};
pub use router::System1Router;

/// Current version of `joiner-core`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");


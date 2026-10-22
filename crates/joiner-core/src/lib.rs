//! # `joiner-core`
//!
//! Master neurosymbolic synthesis orchestrator for the `gaugeblock` suite.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod error;
pub mod filler;
pub mod model;
pub mod router;
pub mod verifier;

pub use error::{JoinerError, Result};
pub use filler::{HoleFillRequest, HoleFiller};
pub use model::{
    AnonymizedSynthesisTrace, CompilerDiagnostic, ExecutionMode, JoinerConfig,
    SynthesisEvent, SynthesisResult, SynthesisStrategy, TelemetryTier, VerificationReport,
};
pub use router::System1Router;
pub use verifier::{AstVerifier, line_col_to_byte_offset};

/// Current version of `joiner-core`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");


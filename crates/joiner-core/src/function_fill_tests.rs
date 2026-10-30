//! Tests for function filling, split from `function_fill.rs` to stay inside the 250-line law.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{DEFAULT_FILL_MAX_TOKENS, FillOptions, VerifyMode, build_request};
use gaugeblock_contracts::ReasoningEffort;

// Deliberate invariant on a constant: the budget must not regress below what a reasoning
// model needs, and clippy flags constant assertions by design.
#[allow(clippy::assertions_on_constants)]
#[test]
fn test_default_budget_leaves_room_for_a_thinking_trace() {
    assert!(
        DEFAULT_FILL_MAX_TOKENS >= 8192,
        "a reasoning model spends budget before emitting content (GB-206)"
    );
    assert_eq!(FillOptions::default().max_tokens, DEFAULT_FILL_MAX_TOKENS);
}

#[test]
fn test_the_budget_reaches_the_request_unchanged() {
    let request = build_request("fill this".to_string(), 4096, None);
    assert_eq!(request.max_tokens, 4096);
    assert_eq!(request.temperature, 0.2);
    assert!(request.system_prompt.is_some());
    assert!(request.grammar_constraint.is_none());
    assert_eq!(
        request.reasoning_effort,
        Some(ReasoningEffort::None),
        "the filler must not pay for a thinking trace (GB-212)"
    );
}

#[test]
fn test_syntax_is_the_default_oracle_and_a_crate_can_be_named() {
    assert_eq!(FillOptions::default().verify, VerifyMode::Syntax);
    assert!(
        !FillOptions::default().apply,
        "nothing is written unless asked"
    );
    let explicit = FillOptions {
        verify: VerifyMode::Crate(std::path::PathBuf::from("/tmp/crate")),
        ..FillOptions::default()
    };
    assert_ne!(explicit.verify, VerifyMode::Syntax);
}

#[test]
fn test_an_explicit_option_overrides_the_default() {
    let options = FillOptions {
        max_tokens: 512,
        ..FillOptions::default()
    };
    assert_eq!(options.max_tokens, 512);
    assert_ne!(options, FillOptions::default());
}

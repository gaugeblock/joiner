//! Tests for the unconstrained function fill (GB-167).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use joiner_core::function_fill::{assess_rust, function_range, strip_code_fences};

#[test]
fn extracts_a_function_by_brace_matching() {
    let source = "fn helper() {}\n\nfn target(a: u8) -> u8 {\n    a + 1\n}\n";
    let (start, end) = function_range(source, "target").expect("found");
    let slice = &source[start..end];

    assert!(slice.starts_with("fn target"), "got: {slice}");
    assert!(slice.ends_with('}'), "the closing brace is included");
    assert!(!slice.contains("helper"), "neighbouring code is excluded");
}

#[test]
fn a_missing_symbol_is_reported_not_guessed() {
    assert!(function_range("fn present() {}", "absent").is_none());
}

#[test]
fn fences_are_stripped_with_or_without_a_language_tag() {
    let fenced = "```rust\nfn a() {}\n```";
    assert_eq!(strip_code_fences(fenced), "fn a() {}");

    let bare = "```\nfn b() {}\n```";
    assert_eq!(strip_code_fences(bare), "fn b() {}");

    let unfenced = "fn c() {}";
    assert_eq!(strip_code_fences(unfenced), "fn c() {}");
}

#[test]
fn syntax_assessment_separates_valid_from_broken_rust() {
    assert!(assess_rust("fn ok() -> u8 { 1 }\n").is_ok());

    let broken = assess_rust("fn broken( {\n");
    assert!(broken.is_err(), "an unclosed signature must not validate");
    assert!(!broken.unwrap_err().is_empty(), "diagnostics are reported");
}

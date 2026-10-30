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

#[test]
fn test_the_prompt_names_the_available_crates() {
    let prompt = super::prepare::build_prompt(
        "grapheme_count",
        "fn grapheme_count(input: &str) -> usize { todo!() }",
        &[],
        &["serde".to_string(), "unicode-segmentation".to_string()],
    );
    assert!(
        prompt.contains(
            "Crates this code may use (declared by the crate): serde, unicode-segmentation"
        ),
        "got: {prompt}"
    );
    assert!(prompt.contains("Do not import anything else."));
}

#[test]
fn test_without_a_crate_the_prompt_says_nothing_about_dependencies() {
    // An empty list must not read as "there are no crates" - syntax-only verification cannot know.
    let prompt = super::prepare::build_prompt("f", "fn f() {}", &[], &[]);
    assert!(
        !prompt.contains("Crates this code may use"),
        "got: {prompt}"
    );
}

#[test]
fn test_declared_dependencies_reads_the_dependency_tables_only() {
    let dir = std::env::temp_dir().join(format!("joiner-deps-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1.0\"\nregex = { version = \"1\" }\n\n[dev-dependencies]\ntempfile = \"3\"\n\n[profile.dev]\nopt-level = 0\n",
    )
    .unwrap();

    let deps = super::prepare::declared_dependencies(&dir);
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(
        deps,
        vec!["regex", "serde", "tempfile"],
        "package keys excluded, sorted"
    );
}

#[test]
fn test_a_missing_manifest_yields_no_dependencies_rather_than_failing() {
    let missing = std::env::temp_dir().join("joiner-no-such-crate-dir");
    assert!(super::prepare::declared_dependencies(&missing).is_empty());
}

#[test]
fn test_the_extracted_context_keeps_the_doc_comment() {
    // GB-224's fixture explains the intended approach in its doc comment; cutting it off is what made
    // the model reimplement the function with a crate the fixture does not have.
    let source = "//! Crate docs.\n\npub fn other() {}\n\n/// Delegates to `crate::text`.\n///\n/// Second line.\n#[inline]\npub fn grapheme_count(input: &str) -> usize {\n    todo!()\n}\n";
    let (start, end) = super::function_range(source, "grapheme_count").expect("range");
    let slice = &source[start..end];

    assert!(slice.contains("Delegates to `crate::text`"), "got: {slice}");
    assert!(
        slice.contains("Second line."),
        "the whole block, not one line"
    );
    assert!(slice.contains("#[inline]"), "attributes belong to the item");
    assert!(
        !slice.contains("pub fn other"),
        "stops at the previous item"
    );
    assert!(slice.trim_end().ends_with('}'));
}

#[test]
fn test_a_function_without_docs_still_extracts() {
    let source = "pub fn a() {}\n\nfn b() -> usize { 1 }\n";
    let (start, end) = super::function_range(source, "b").expect("range");
    assert_eq!(&source[start..end], "fn b() -> usize { 1 }");
}

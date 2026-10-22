#![allow(clippy::expect_used, clippy::unwrap_used)]

use dado_core::splice::{SpliceOptions, splice_pattern};
use dado_core::{DadoPattern, TypedHole};
use joiner_core::model::ExecutionMode;
use joiner_core::verifier::{AstVerifier, line_col_to_byte_offset};
use miter_engine::Language;
use std::collections::BTreeMap;
use tempfile::tempdir;

fn sample_rust_pattern() -> DadoPattern {
    DadoPattern {
        pattern_id: "test.pattern.calc.v1".into(),
        language: Language::Rust,
        description: "Arithmetic calculation pattern".into(),
        template_source: "pub fn compute() -> i32 {\n    let val = ?[val: i32 @ expression];\n    val + ?[delta: i32 @ expression]\n}".into(),
        required_imports: vec![],
        holes: vec![
            TypedHole::new("val", "i32", "expression"),
            TypedHole::new("delta", "i32", "expression"),
        ],
        canonical_example_fills: BTreeMap::new(),
    }
}

#[test]
fn test_line_col_to_byte_offset() {
    let source = "line1\nline2\nline3";
    // line 1, col 1 -> 0
    assert_eq!(line_col_to_byte_offset(source, 1, 1), 0);
    // line 2, col 1 -> 6 (after "line1\n")
    assert_eq!(line_col_to_byte_offset(source, 2, 1), 6);
    // line 2, col 5 -> 10
    assert_eq!(line_col_to_byte_offset(source, 2, 5), 10);
}

#[test]
fn test_span_attribution_to_exact_hole() {
    let pattern = sample_rust_pattern();
    let mut fills = BTreeMap::new();
    fills.insert("val".to_string(), "42".to_string());
    fills.insert("delta".to_string(), "100".to_string());

    let opts = SpliceOptions::default();
    let splice_res = splice_pattern(&pattern, &fills, &opts).expect("splice pattern");

    assert!(splice_res.hole_spans.contains_key("val"));
    assert!(splice_res.hole_spans.contains_key("delta"));

    let (val_start, val_end) = splice_res.hole_spans["val"];
    let (delta_start, _delta_end) = splice_res.hole_spans["delta"];

    // Offset inside "val"
    let inside_val = val_start + 1;
    assert!(inside_val <= val_end);

    let spans = vec![miter_harness::DiagnosticSpan {
        file_path: "src/lib.rs".into(),
        byte_start: inside_val,
        byte_end: inside_val + 1,
        line_start: 2,
        line_end: 2,
        column_start: 15,
        column_end: 16,
    }];

    // Test byte-level attribution
    let mut attributed = None;
    for span in &spans {
        for (hole_id, (start, end)) in &splice_res.hole_spans {
            if span.byte_start >= *start && span.byte_start <= *end {
                attributed = Some(hole_id.clone());
            }
        }
    }
    assert_eq!(attributed, Some("val".to_string()));

    // Test inside "delta"
    let inside_delta = delta_start;
    let mut delta_attributed = None;
    for (hole_id, (start, end)) in &splice_res.hole_spans {
        if inside_delta >= *start && inside_delta <= *end {
            delta_attributed = Some(hole_id.clone());
        }
    }
    assert_eq!(delta_attributed, Some("delta".to_string()));
}

#[tokio::test]
async fn test_ast_verifier_syntax_failure() {
    let pattern = sample_rust_pattern();
    let mut fills = BTreeMap::new();
    // Fill with completely invalid syntax tokens
    fills.insert("val".to_string(), "{ invalid syntax !!!".to_string());
    fills.insert("delta".to_string(), "10".to_string());

    let temp = tempdir().expect("temp dir");
    let verifier = AstVerifier;

    let (report, _splice, _affected) = verifier
        .verify(
            temp.path(),
            "src/calc.rs",
            &pattern,
            &fills,
            ExecutionMode::InPlace,
            false,
        )
        .await
        .expect("verify");

    assert!(!report.ast_valid, "Invalid syntax must fail AST check");
    assert!(!report.typecheck_passed);
    assert!(!report.diagnostics.is_empty());
    assert_eq!(
        report.diagnostics[0].code.as_deref(),
        Some("AST_SYNTAX_ERROR")
    );
}

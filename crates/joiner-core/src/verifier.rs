//! Verification runner with AST check, dual-mode execution, and diagnostic span attribution.

use crate::error::Result;
use crate::model::{CompilerDiagnostic, ExecutionMode, VerificationReport};
use dado_core::splice::{SpliceOptions, splice_pattern};
use dado_core::{DadoPattern, SpliceResult};
use miter_harness::{CompilerVerifier, DiagnosticSpan, Toolchain, VerificationOutcome};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;
use tokio::fs;
use tracing::info;

/// Verification engine coordinating splicing, AST validation, compiler checks, and tests.
#[derive(Debug, Clone, Default)]
pub struct AstVerifier;

impl AstVerifier {
    /// Splices filled holes and verifies syntax, compilation, and tests.
    pub async fn verify(
        &self,
        workspace_root: &Path,
        rel_target_file: &str,
        pattern: &DadoPattern,
        fills: &BTreeMap<String, String>,
        mode: ExecutionMode,
        run_tests: bool,
    ) -> Result<(VerificationReport, SpliceResult, Option<String>)> {
        let start = Instant::now();
        let opts = SpliceOptions {
            available_imports: None,
            include_imports_header: true,
        };

        let splice_res = splice_pattern(pattern, fills, &opts)?;

        // 1. In-memory AST syntax validation from dado
        if !splice_res.ast_valid {
            let diags = if splice_res.diagnostics.is_empty() {
                vec![CompilerDiagnostic {
                    file: rel_target_file.to_string(),
                    line: 1,
                    column: 1,
                    code: Some("AST_SYNTAX_ERROR".into()),
                    message: "AST syntax validation failed with syntax errors".into(),
                    suggestion: None,
                }]
            } else {
                splice_res
                    .diagnostics
                    .iter()
                    .map(|msg| CompilerDiagnostic {
                        file: rel_target_file.to_string(),
                        line: 1,
                        column: 1,
                        code: Some("AST_SYNTAX_ERROR".into()),
                        message: msg.clone(),
                        suggestion: None,
                    })
                    .collect()
            };

            let report = VerificationReport {
                ast_valid: false,
                typecheck_passed: false,
                tests_passed: None,
                diagnostics: diags,
                duration_ms: start.elapsed().as_millis() as u64,
            };
            return Ok((report, splice_res, None));
        }

        // 2. Prepare target file write
        let (exec_dir, full_path) = match mode {
            ExecutionMode::InPlace => (
                workspace_root.to_path_buf(),
                workspace_root.join(rel_target_file),
            ),
            ExecutionMode::Sandbox => {
                let sbox_dir = workspace_root.join(".miter/worktrees/joiner-verify");
                fs::create_dir_all(&sbox_dir).await?;
                let path = sbox_dir.join(rel_target_file);
                (sbox_dir, path)
            }
        };

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&full_path, &splice_res.rendered_code).await?;

        // 3. Compiler check
        let (typecheck_passed, diagnostics, outcome_spans) =
            self.run_compiler_check(&exec_dir).await?;

        // 4. Attribute diagnostic to a specific TypedHole span
        let affected_hole = if !typecheck_passed {
            self.attribute_failure(&splice_res, &diagnostics, &outcome_spans)
        } else {
            None
        };

        // 5. Test runner execution
        let tests_passed = if typecheck_passed && run_tests {
            Some(self.run_unit_tests(&exec_dir).await?)
        } else {
            None
        };

        let report = VerificationReport {
            ast_valid: true,
            typecheck_passed,
            tests_passed,
            diagnostics,
            duration_ms: start.elapsed().as_millis() as u64,
        };

        Ok((report, splice_res, affected_hole))
    }

    async fn run_compiler_check(
        &self,
        exec_dir: &Path,
    ) -> Result<(bool, Vec<CompilerDiagnostic>, Vec<DiagnosticSpan>)> {
        let verifier = CompilerVerifier::new(Toolchain::Cargo);
        let outcome = verifier.verify(exec_dir).await?;

        match outcome {
            VerificationOutcome::Success { .. } => Ok((true, vec![], vec![])),
            VerificationOutcome::DiagnosticFailure {
                error_code,
                message,
                spans,
                ..
            } => {
                let mut diags = Vec::new();
                for span in &spans {
                    diags.push(CompilerDiagnostic {
                        file: span.file_path.clone(),
                        line: span.line_start,
                        column: span.column_start,
                        code: Some(error_code.clone()),
                        message: message.clone(),
                        suggestion: None,
                    });
                }
                if diags.is_empty() {
                    diags.push(CompilerDiagnostic {
                        file: "unknown".into(),
                        line: 1,
                        column: 1,
                        code: Some(error_code),
                        message,
                        suggestion: None,
                    });
                }
                Ok((false, diags, spans))
            }
        }
    }

    async fn run_unit_tests(&self, exec_dir: &Path) -> Result<bool> {
        let output = tokio::process::Command::new("cargo")
            .arg("test")
            .arg("--quiet")
            .current_dir(exec_dir)
            .output()
            .await?;
        Ok(output.status.success())
    }

    fn attribute_failure(
        &self,
        splice_res: &SpliceResult,
        diags: &[CompilerDiagnostic],
        spans: &[DiagnosticSpan],
    ) -> Option<String> {
        // First try byte-accurate DiagnosticSpan from miter-harness
        for span in spans {
            for (hole_id, (start, end)) in &splice_res.hole_spans {
                if span.byte_start >= *start && span.byte_start <= *end {
                    info!(hole = %hole_id, "Attributed failure to hole via DiagnosticSpan");
                    return Some(hole_id.clone());
                }
            }
        }

        // Fallback to line-column offset mapping
        for diag in diags {
            let offset = line_col_to_byte_offset(&splice_res.rendered_code, diag.line, diag.column);
            for (hole_id, (start, end)) in &splice_res.hole_spans {
                if offset >= *start && offset <= *end {
                    info!(hole = %hole_id, "Attributed failure to hole via line:column");
                    return Some(hole_id.clone());
                }
            }
        }

        None
    }
}

/// Converts a 1-based (line, column) coordinate to an absolute UTF-8 byte offset.
pub fn line_col_to_byte_offset(source: &str, target_line: usize, target_col: usize) -> usize {
    for (idx, line) in source.split('\n').enumerate() {
        if idx + 1 == target_line {
            let col_offset = target_col.saturating_sub(1);
            return (line.as_ptr() as usize - source.as_ptr() as usize)
                + col_offset.min(line.len());
        }
    }
    source.len()
}

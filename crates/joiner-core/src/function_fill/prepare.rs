//! Source surgery and candidate verification for function filling, split from
//! `function_fill.rs` to respect the 250-line law.
//!
//! These are the mechanical halves of a fill: locating the hole, unwrapping a fenced answer,
//! judging whether it parses, and compiling a candidate in place with the tree restored after.

pub fn function_range(source: &str, symbol: &str) -> Option<(usize, usize)> {
    let start = source.find(&format!("fn {symbol}"))?;
    let open = source[start..].find('{')? + start;

    let mut depth = 0usize;
    for (offset, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start, open + offset + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// Drops markdown code fences, keeping the fenced body.
#[must_use]
pub fn strip_code_fences(text: &str) -> String {
    let trimmed = text.trim();
    let Some(open) = trimmed.find("```") else {
        return trimmed.to_string();
    };
    let after_open = &trimmed[open + 3..];
    let body = after_open
        .find('\n')
        .map_or(after_open, |newline| &after_open[newline + 1..]);
    match body.rfind("```") {
        Some(close) => body[..close].trim_end().to_string(),
        None => body.trim_end().to_string(),
    }
}

/// `Ok(())` when the source parses with zero syntax errors, otherwise the diagnostics.
pub fn assess_rust(source: &str) -> std::result::Result<(), Vec<String>> {
    match miter_engine::polyglot::syntax_diagnostics(source, miter_engine::polyglot::Language::Rust)
    {
        Ok(diagnostics) if diagnostics.is_empty() => Ok(()),
        Ok(diagnostics) => Err(diagnostics
            .iter()
            .map(|d| format!("L{}:C{} {:?}", d.line, d.column, d.kind))
            .collect()),
        Err(error) => Err(vec![error.to_string()]),
    }
}

/// Restores a file when dropped, so a candidate can be compiled in place without risking the tree.
pub(super) struct RestoreGuard {
    path: std::path::PathBuf,
    original: String,
    keep: bool,
}

impl Drop for RestoreGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::write(&self.path, &self.original);
        }
    }
}

/// Compiles the crate at `crate_dir` with `miter verify` and returns the diagnostics if it fails.
pub(super) fn verify_in_crate(crate_dir: &std::path::Path) -> std::result::Result<(), Vec<String>> {
    let output = std::process::Command::new("miter")
        .arg("verify")
        .arg("--path")
        .arg(crate_dir)
        .arg("--json")
        .output()
        .map_err(|error| vec![format!("miter verify could not run: {error}")])?;

    let body = String::from_utf8_lossy(&output.stdout);
    let parsed: Option<serde_json::Value> = serde_json::from_str(body.trim()).ok();
    let outcome = parsed
        .as_ref()
        .and_then(|value| value.get("outcome"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    if outcome == "success" {
        return Ok(());
    }

    let message = parsed
        .as_ref()
        .and_then(|value| value.get("message"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| String::from_utf8_lossy(&output.stderr).to_string());
    Err(vec![format!("compile: {message}")])
}

/// Writes `candidate` into `file`, compiles the crate, and restores the file unless `keep`.
pub(super) fn compiles_in_place(
    file: &std::path::Path,
    candidate: &str,
    crate_dir: &std::path::Path,
    keep: bool,
) -> std::result::Result<(), Vec<String>> {
    let original = std::fs::read_to_string(file).unwrap_or_default();
    let mut guard = RestoreGuard {
        path: file.to_path_buf(),
        original,
        keep: false,
    };
    if let Err(error) = std::fs::write(file, candidate) {
        return Err(vec![format!("candidate could not be written: {error}")]);
    }
    let verdict = verify_in_crate(crate_dir);
    guard.keep = keep && verdict.is_ok();
    drop(guard);
    verdict
}

/// Names of the dependencies declared by the crate at `dir`, sorted and deduplicated.
///
/// A fill that cannot see the manifest reaches for the crate that would make the code idiomatic -
/// `unicode_segmentation` for grapheme counting - and then fails verification with an unresolved
/// import, which no amount of retrying fixes (`GB-224`). Reading the manifest costs one file read and
/// turns that into a prompt that names what exists.
///
/// Deliberately a line scan rather than a TOML parse: the only thing needed is the key set of the
/// dependency tables, and a full parse would need a parser dependency in a crate that has none.
pub(super) fn declared_dependencies(dir: &std::path::Path) -> Vec<String> {
    let manifest = dir.join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(manifest) else {
        return Vec::new();
    };

    let mut names = Vec::new();
    let mut in_dependencies = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = matches!(
                trimmed,
                "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
            );
            continue;
        }
        if !in_dependencies || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            let name = name.trim();
            if !name.is_empty() && !names.iter().any(|existing| existing == name) {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    names
}

/// Builds the fill prompt.
///
/// Pure so the shape is testable without a provider. `available` is empty when the caller is not
/// verifying inside a crate, in which case the prompt says nothing about dependencies rather than
/// implying there are none.
pub(super) fn build_prompt(
    label: &str,
    context: &str,
    diagnostics: &[String],
    available: &[String],
) -> String {
    let diagnostic_block = if diagnostics.is_empty() {
        "none".to_string()
    } else {
        diagnostics.join(
            "
",
        )
    };
    let availability = if available.is_empty() {
        String::new()
    } else {
        format!(
            "

Crates this code may use (declared by the crate): {}. Do not import anything else.",
            available.join(", ")
        )
    };
    format!(
        "Fill {label} in Rust.

Current code:
```rust
{context}
```{availability}

Syntax diagnostics from the previous attempt:
{diagnostic_block}

Return only the complete replacement code inside one ```rust fence."
    )
}

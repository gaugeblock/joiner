# GB-93: Fix missing reasoning_effort on InferenceRequest in joiner-core

## Context & Root Cause
In `sawhorse-core`, `InferenceRequest` gained a new field:
`pub reasoning_effort: Option<ReasoningEffort>`.
In `joiner-core`, raw struct literals were used in three places without `reasoning_effort`:
1. `crates/joiner-core/src/filler.rs:133`
2. `crates/joiner-core/src/oracle.rs:77`
3. `crates/joiner-core/src/router.rs:153`

## Implementation Steps
1. In `joiner/crates/joiner-core/src/filler.rs`:
   - Supply `reasoning_effort: None` in `InferenceRequest` initialization.
2. In `joiner/crates/joiner-core/src/oracle.rs`:
   - Supply `reasoning_effort: None` in `InferenceRequest` initialization.
3. In `joiner/crates/joiner-core/src/router.rs`:
   - Supply `reasoning_effort: None` in `InferenceRequest` initialization.
4. Verify:
   - Check LOC limits (<=250 LOC).
   - Run `cargo check -p joiner-core` and `cargo test -p joiner-core`.
   - Run Tier 2 checks: `cargo fmt --check -p joiner-core`, `cargo clippy -p joiner-core --all-targets -- -D warnings`, `cargo test -p joiner-core`.
5. Commit atomically: `fix(joiner-core): initialize reasoning_effort in InferenceRequest callsites`.


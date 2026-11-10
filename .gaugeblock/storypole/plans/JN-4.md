# Task Plan: GB-165 — Live Deterministic Trace Capture

> **Key:** `GB-165` **Scope:** `storypole` (owner), `common` (schema), `joiner` (consumer) **Status:** Slice 1 done

## 1. Decisions (settled, do not relitigate)

**`storypole verify <KEY>` is the single owner of post-task capture.** `outfeed gate` is a
*stateless, repeatable* check run many times mid-branch; emitting there would write duplicate
intermediate traces before a task is finished. `verify` is the one state transition where
`base_commit_sha`, `head_commit_sha`, `commit_shas` and the task context are known and immutable.

**Idempotency key: `(task_key, head_commit_sha)`.** One trace per verified head, even if `verify` is
re-run. A re-run with a new head is a new trace.

**No storypole → joiner Cargo edge.** Trace capture must not create a dependency cycle. The record
type already exists in `joiner-core` (`AnonymizedSynthesisTrace`, `TelemetryTier::{Bronze,Silver,Gold}`,
`assign_telemetry_tier`), so **move those two types into `gaugeblock-contracts`** and have both
storypole and joiner use them. Inventing a second schema on the storypole side is the drift that
`GB-148` was made of.

## 2. Slices

1. **`.gitignore` for derived state — done** (`harness/feat-live-trace-capture`). Six derived paths
   ignored, three canonical paths still tracked, verified with `git check-ignore` both ways.
2. **Schema move** into `gaugeblock-contracts`; `joiner-core` re-exports so its paths keep working.
3. **Spool writer in `storypole-core`**: append one JSONL line to
   `<canonical .gaugeblock>/spool/outbox.jsonl`, creating the directory; scrubbed via the same rules
   joiner applies; returns the record so the caller can log it.
4. **Hook in `verify_task`**: after the receipt is recorded, extract the functions added in
   `base..head` (git diff + miter symbol lookup), emit one Silver trace per function, and degrade to
   a `[skip]` warning when `miter` is absent rather than failing verification.
5. **Dogfood protocol** codified in `AGENTS.md`: `dado contract` for new wire types,
   `dado render` + `batten validate` for batches.

## 3. Acceptance

* Verifying `GB-165` itself writes the first live Silver trace to `.gaugeblock/spool/outbox.jsonl`.
* A second `verify` on the same head writes nothing (idempotency).
* Traces contain no secrets: `CLINE_API_KEY` and friends never appear in a record.

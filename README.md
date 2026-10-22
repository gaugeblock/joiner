# `gaugeblock/joiner`

Master neurosymbolic synthesis orchestrator for the [gaugeblock](https://gaugeblock.dev) suite.

`joiner` is the autonomous execution engine that wrings together `storypole`, `chalkline`, `dado`, `batten`, `sawhorse`, and `miter` to pull ready tasks, select AST patterns, fill grammar-masked holes, verify compilation and tests, self-repair compiler diagnostics, and emit anonymized telemetry for `rabbet`.

## Architecture Overview

```text
                  storypole::TaskNode (Ready)
                             │
                             ▼
                ┌─────────────────────────┐
                │     System 1 Router     │
                │       (router.rs)       │
                │ - Deterministic Registry│
                │ - sawhorse System1Router│
                │ - Surgical AST / Oracle │
                └────────────┬────────────┘
                             │
                             ▼
                ┌─────────────────────────┐
                │      Hole Filler        │
                │      (filler.rs)        │
                │ For each TypedHole:     │
                │  1. batten CFG mask     │
                │  2. sawhorse completion │
                │  3. batten validation   │
                └────────────┬────────────┘
                             │
                             ▼
                ┌─────────────────────────┐
                │   AST & Test Verifier   │
                │      (verifier.rs)      │
                │  1. dado::splice()      │
                │  2. AST syntax check    │
                │  3. In-Place or Sandbox │
                │  4. cargo check / test  │
                └────────────┬────────────┘
                             │
            ┌────────────────┴────────────────┐
            ▼ (If Failed)                     ▼ (If Passed)
    ┌────────────────┐                ┌───────────────┐
    │  Self-Repair / │                │   storypole   │
    │  Oracle (3x)   │                │  (Verified)   │
    └────────────────┘                └───────┬───────┘
                                              │
                                              ▼
                                      ┌───────────────┐
                                      │ Telemetry     │
                                      │ Spool Outbox  │
                                      │ (Bronze/Gold) │
                                      └───────────────┘
```

## Features

- **Dual-Mode Execution:** Direct working tree mutation (`--in-place`) or isolated git worktree sandboxing (`--sandbox` via `miter-harness`).
- **Two-Stage Hybrid Router:** Exact AST pattern lookup with LLM classification fallback.
- **Parallel Default Hole Filling:** Fast concurrent completion of independent holes with sequential fallback.
- **Progressive Self-Repair:** Attribution of compiler diagnostics to exact hole byte spans before escalating to unconstrained frontier Oracle.
- **Spool Outbox Telemetry:** Zero PII / secret scrubbing via `miter` and batch-exportable JSONL spool for `rabbet`.
- **Benchwork Daemon:** Axum HTTP server with REST, WebSocket, and SSE streaming.

## License

Apache-2.0


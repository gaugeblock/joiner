# `joiner` — Master Neurosymbolic Synthesis Orchestrator

[![Version](https://img.shields.io/badge/version-0.1.0--beta.2-blue.svg)](https://gaugeblock.dev)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![gaugeblock](https://img.shields.io/badge/suite-gaugeblock-orange.svg)](https://github.com/gaugeblock)
[![Documentation](https://img.shields.io/badge/docs-9--doc%20topology-green.svg)](ARCHITECTURE.md)

Part of the **[`gaugeblock`](https://gaugeblock.dev)** neurosymbolic code synthesis ecosystem.

> In traditional woodworking, a **joiner** is the master craftsman who fits separate wooden members together into intricate, structurally sound assemblies—doors, sashes, stairs, and furniture—using precise mortise-and-tenon, dovetail, and lap joints. In the `gaugeblock` suite, `joiner` orchestrates the entire synthesis pipeline: it links `storypole`, `chalkline`, `dado`, `batten`, `sawhorse`, and `miter` into an autonomous closed loop that pulls ready tasks, selects patterns, fills grammar-masked holes, verifies compilation and tests, self-repairs compiler diagnostics, and streams verified training data to `rabbet`.

---

## 1. Architecture Overview

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

---

## 2. Core Capabilities

* **Dual-Mode Execution:** Direct working tree mutation (`--in-place`) or isolated git worktree sandboxing (`--sandbox` via `miter-harness`).
* **Two-Stage Hybrid Router:** Exact AST pattern lookup with LLM classification fallback to minimize costly unconstrained generation.
* **Parallel Default Hole Filling:** Fast concurrent completion of independent holes with deterministic sequential fallback.
* **Progressive Self-Repair Loop:** Attributions of compiler diagnostics directly to exact hole byte spans before escalating to unconstrained frontier Oracle.
* **Spool Outbox Telemetry:** Zero PII / secret scrubbing via `miter` and batch-exportable JSONL spool (`.gaugeblock/spool/`) for downstream `rabbet` fine-tuning.
* **Axum Daemon & WebSocket Streaming:** Embedded HTTP and event streaming for live execution visualization in `benchwork`.

---

## 3. Workspace Crate Layout

* **`joiner`**: Facade crate re-exporting `joiner-core`.
* **`crates/joiner-core`**: Core orchestration engine, System 1 router, hole filler, compiler verifier, and telemetry spooler.
* **`crates/joiner-cli`**: Clap binary entrypoint (`joiner`).
* **`crates/joiner-mcp`**: JSON-RPC 2.0 stdio Model Context Protocol (MCP) server.

---

## 4. Installation & CLI Quickstart

```bash
# Install globally via cargo
cargo install --path crates/joiner-cli

# 1. Pull and execute the next ready task from Storypole in an isolated worktree
joiner run-next --sandbox

# 2. Execute synthesis directly on active working tree
joiner run-next --in-place

# 3. Target a specific task key
joiner synthesize --task GB-42 --in-place

# 4. Inspect current orchestrator status and daemon health
joiner status
```

---

## 5. Model Context Protocol (MCP) Reference

| Tool | Parameters | Description |
|---|---|---|
| `joiner_run_next` | `sandbox?` | Lease and execute the next unblocked task from Storypole. |
| `joiner_synthesize` | `task_key, in_place?` | Execute targeted neurosymbolic synthesis on a specific task node. |
| `joiner_status` | `None` | Query active synthesis leases, queue health, and spool outbox metrics. |

---

## 6. Repository Documentation

`joiner` adheres strictly to the canonical **9-Document Architecture**:
* **[`CONTRIBUTING.md`](./CONTRIBUTING.md):** Local toolchain prerequisites, sandbox testing, and test execution.
* **[`ARCHITECTURE.md`](./ARCHITECTURE.md):** Orchestrator pipeline, self-repair state machine, and router mechanics.
* **[`SPEC.md`](./SPEC.md):** Synthesis state transitions, repair invariants, and telemetry schema contracts.
* **[`AGENTS.md`](./AGENTS.md):** Autonomous systems directives, <=250 LOC limits, and zero-unwrap rules.
* **[`CHANGELOG.md`](./CHANGELOG.md):** Chronological historical release entries and unreleased changes.

---

## 7. Verification & Development Directives

* **File Size Ceiling:** All `.rs` files must strictly remain under 250 LOC.
* **Tiered Verification Matrix:**
  * Tier 1: `cargo check -p joiner-core` (<2s).
  * Tier 2: `cargo fmt --check -p joiner-core && cargo clippy -p joiner-core && cargo test -p joiner-core`.
  * Tier 3: `chalkline docs check -d .` before task verification.

---

## 8. License

Licensed under the [Apache License, Version 2.0](LICENSE).
Part of the [`gaugeblock`](https://gaugeblock.dev) suite.

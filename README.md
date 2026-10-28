# `joiner` — Master Neurosymbolic Synthesis Orchestrator

[![Version](https://img.shields.io/badge/version-0.1.0--beta.3-blue.svg)](https://gaugeblock.dev)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![gaugeblock](https://img.shields.io/badge/suite-gaugeblock-orange.svg)](https://github.com/gaugeblock)
[![Documentation](https://img.shields.io/badge/docs-9--doc%20topology-green.svg)](ARCHITECTURE.md)

Part of the **[`gaugeblock`](https://gaugeblock.dev)** neurosymbolic code synthesis ecosystem.

> In traditional woodworking, a **joiner** is the master craftsman who fits separate wooden members together into intricate, structurally sound assemblies—doors, sashes, stairs, and furniture—using precise mortise-and-tenon, dovetail, and lap joints. In the `gaugeblock` suite, `joiner` orchestrates the entire synthesis pipeline: it links `storypole`, `chalkline`, `dado`, `batten`, `sawhorse`, and `miter` into an autonomous closed loop that pulls ready tasks, selects patterns, fills grammar-masked holes, verifies compilation and tests, self-repairs compiler diagnostics, and streams verified training data to `rabbet`.

---

## Why `joiner`?

Current AI coding assistants operate in an unverified open loop: they generate arbitrary text diffs, assume success, and leave broken builds for human developers to debug.

| Without `joiner` | With `joiner` |
|---|---|
| **Open-Loop Hallucinations:** Agents mark tasks done based on internal confidence, leaving behind compilation failures, broken imports, and failing tests. | **Closed-Loop Verification Gate:** `joiner` never marks a task verified without real compiler execution (`cargo check`, `tsc`) and test passes (`exit_code == 0`). |
| **Monolithic Token Waste:** Generating entire files from scratch wastes 85%+ of tokens on predictable framework boilerplate and syntax. | **Two-Stage System 1 Routing:** Routes routine tasks to verified `dado` AST patterns with `batten` token masks, reserving frontier LLMs strictly for novel algorithms. |
| **Destructive Self-Repair:** When a build breaks, agents re-prompt models to re-write whole files from scratch, frequently fixing one bug while introducing two more. | **Localized Hole-Level Repair:** Maps compiler diagnostic spans directly to specific typed holes, re-prompting only the broken logic with a strict 3x iteration limit. |
| **Polluted Working Tree:** Speculative code changes dirty git working trees, causing merge conflicts and broken developer environments when experiments fail. | **Ephemeral Git Worktree Sandboxing:** `joiner --sandbox` executes synthesis and compiler checks in an isolated worktree, leaving the user's branch 100% clean. |
| **Lost Training Flywheel:** Autonomous coding sessions evaporate into lost chat transcripts without improving underlying models. | **Automated Telemetry Spooling:** Emits privacy-scrubbed Bronze, Silver, and Gold traces to `.gaugeblock/spool/` to train smaller, specialized open-weight models in `rabbet`. |

---

## 1. System Architecture

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

## 2. CLI Commands Reference

`joiner` provides a unified command set for standalone terminal execution, background daemon monitoring, and agent workflows. Pass `--json` to any command for structured output.

| Command | Description | MCP Tool |
|---|---|---|
| `run-next` | Claim and synthesize the next topologically ready task from `storypole`. | `joiner_run_next` |
| `synthesize <KEY>` | Synthesize, verify, and repair code for a specific target task key. | `joiner_synthesize` |
| `daemon` | Launch the HTTP/WebSocket/SSE server on port 7680 for `benchwork` visualization. | — |
| `telemetry` | Inspect pending outbox traces or export them for `rabbet` ingestion. | — |
| `mcp` | Launch the Model Context Protocol (JSON-RPC 2.0 stdio) server. | — |

---

## 3. Model Context Protocol (MCP) Reference

When operating inside AI agents (Antigravity, Claude Code, Cursor, Cline), `joiner` exposes 3 core tools:

| Tool | Parameters | Description |
|---|---|---|
| `joiner_run_next` | `dir?, sandbox?, sequential?` | Claim the next unblocked ready task, execute synthesis, and verify. |
| `joiner_synthesize` | `task_id, dir?, sandbox?` | Synthesize and verify code for a specific Storypole task. |
| `joiner_status` | `dir?` | Retrieve daemon health, active synthesis progress, and outbox spool metrics. |

---

## 4. Installation & Quickstart

```bash
# Install globally via cargo
cargo install --path crates/joiner-cli

# 1. Synthesize next ready task in an isolated git worktree sandbox
joiner run-next --sandbox

# 2. Synthesize a specific task directly in the active working directory
joiner synthesize GB-12

# 3. Launch the Axum daemon for benchwork visual studio integration
joiner daemon --port 7680

# 4. Export verified telemetry outbox traces to stdout
joiner telemetry --export --limit 50
```

---

## 5. Canonical 9-Document Architecture

`joiner` adheres strictly to the canonical 9-Document Architecture:
* **[`README.md`](./README.md):** Public elevator pitch, CLI reference, and quickstart.
* **[`CONTRIBUTING.md`](./CONTRIBUTING.md):** Developer environment setup, Rust 2024 toolchain, and test execution.
* **[`PRODUCT.md`](./PRODUCT.md):** User personas, JTBD, closed-loop verification metrics, and problem statements.
* **[`ARCHITECTURE.md`](./ARCHITECTURE.md):** Subsystem topologies, synthesis pipelines, and crate boundaries.
* **[`SPEC.md`](./SPEC.md):** Formal synthesis invariants, repair lifecycles, and type contracts.
* **[`DESIGN.md`](./DESIGN.md):** Architectural Decision Records (ADRs) and trade-off rationales.
* **[`ROADMAP.md`](./ROADMAP.md):** Strategic milestones, active beta capabilities, and horizons.
* **[`CHANGELOG.md`](./CHANGELOG.md):** Chronological historical release logs and unreleased entries.
* **[`AGENTS.md`](./AGENTS.md):** Non-negotiable engineering directives, 250-line file ceiling, and verification matrix.

---

## 6. License

Licensed under the [Apache License, Version 2.0](LICENSE).

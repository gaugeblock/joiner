# System Architecture — `joiner`

> **Crates:** `joiner`, `joiner-core`, `joiner-cli`, `joiner-mcp`  
> **Orchestration Loop:** Closed-Loop Neurosymbolic Code Synthesis  
> **Daemon Mode:** Embedded Axum Server (HTTP, WebSocket, SSE) on Port 7680

---

## 1. System Overview & Dataflow

`joiner` is the master neurosymbolic synthesis engine of the `gaugeblock` ecosystem. It connects task management (`storypole`), AST templates (`dado`), grammar constraints (`batten`), model inference (`sawhorse`), and verification sandboxes (`miter`) into an autonomous, closed execution loop.

```mermaid
flowchart TD
    subgraph Backlog["1. Task Ingestion & Claim"]
        SP["storypole (Task Backlog)"] -->|next ready task| JN["JoinerEngine"]
        JN -->|claim lease (branch + SHA)| SP
    end

    subgraph Routing["2. System 1 Strategy Routing"]
        JN --> RT["System1Router"]
        RT -->|match standard idiom| PAT["dado (AST Pattern + Typed Holes)"]
        RT -->|novel logic / no match| ORC["OracleRunner (Frontier LLM)"]
    end

    subgraph Filling["3. Constrained Hole In-Filling"]
        PAT --> FL["HoleFiller"]
        FL -->|extract active scope| MT_S["miter (Lexical Scope)"]
        FL -->|compile GBNF / Lark| BT["batten (Grammar Compiler)"]
        BT -->|constrained logits| SH["sawhorse (3-Mode LLM)"]
        SH -->|candidate tokens| PRE["batten validate (<50μs)"]
        PRE -->|splice into template| DD_R["dado render (Tree-sitter)"]
    end

    subgraph Verification["4. Verification & Repair Loop"]
        DD_R --> VER["AstVerifier"]
        ORC --> VER
        VER -->|apply in sandbox worktree| WT["miter-harness (git worktree)"]
        WT -->|cargo check / test| CMP["Compiler & Test Runner"]
        CMP -->|exit 0: success| CERT["storypole verify"]
        CMP -->|exit != 0: diagnostics| REP["Self-Repair Engine (<= 3x)"]
        REP -->|re-fill specific hole| FL
        REP -->|escalate after 3x failures| ORC
    end

    subgraph Telemetry["5. Telemetry & Continuous Learning"]
        CERT --> SPL["FileSpoolSink (.gaugeblock/spool/)"]
        SPL -->|outbox sync| RB["rabbet (Training Foundry)"]
    end

    subgraph Interfaces["6. Presentation & IPC"]
        JN --> CLI["joiner-cli (Terminal / JSON)"]
        JN --> MCP["joiner-mcp (JSON-RPC 2.0 Stdio)"]
        JN --> DAE["Axum Daemon (HTTP / WS / SSE)"]
        DAE --> BW["benchwork (Visual Web UI)"]
    end
```

---

## 2. Telemetry Spool Outbox Architecture

`joiner` decouples code synthesis from training telemetry ingestion via an asynchronous, append-only outbox:

```text
.gaugeblock/
└── spool/
    ├── outbox.jsonl           # Append-only JSONL stream of AnonymizedSynthesisTrace records
    └── synced.cursor          # High-water mark tracking records ingested by rabbet
```

* **Bronze Tier:** Synthesized pattern passed syntax and type checking on attempt #1.
* **Silver Tier:** Passed full compiler and unit test suites on attempt #1.
* **Gold Tier:** Required localized self-repair or Oracle fallback before passing all tests (yields DPO preference pairs for model fine-tuning).
* **Diamond Tier:** Code survived unchanged in git `main` across subsequent commits without revert.

---

## 3. Crate & Package Topology

### `joiner`
- **Path:** Root package (`src/lib.rs`)
- **Role:** Facade library crate re-exporting `joiner-core` engines, models, and error types.

### `joiner-core`
- **Path:** `crates/joiner-core`
- **Role:** Core orchestration engine. Contains `JoinerEngine`, `System1Router`, `HoleFiller`, `AstVerifier`, self-repair loops, Axum daemon routes, and telemetry spool sinks. Contains zero CLI Clap parsing code.

### `joiner-cli`
- **Path:** `crates/joiner-cli`
- **Role:** Command-line binary (`joiner`). Parses command options, dispatches `run-next`, `synthesize`, `daemon`, `mcp`, and `telemetry` subcommands.

### `joiner-mcp`
- **Path:** `crates/joiner-mcp`
- **Role:** Model Context Protocol stdio server. Exposes tools (`joiner_run_next`, `joiner_synthesize`, `joiner_status`) to AI coding agents.

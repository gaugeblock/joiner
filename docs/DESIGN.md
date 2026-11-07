# Architectural Design & Decision Records — `joiner`

> **Crates:** `joiner`, `joiner-core`, `joiner-cli`, `joiner-mcp`  
> **Status:** Active Architectural Decision Records (ADRs)

---

## Architectural Decision Records (ADRs)

### ADR-001: Closed-Loop Verification-Gated Synthesis
- **Status:** Accepted
- **Context:** AI coding agents frequently claim a task is completed based on intermediate model confidence, only for developers to discover broken builds or failing unit tests after merging.
- **Decision:** Mandate that `joiner` never marks a task `Verified` in `storypole` without executing real compiler and test suite subprocesses and confirming `exit_code == 0`. Record a cryptographic `VerificationReceipt` linking verified commit SHAs to the task.
- **Consequences:** 100% reliable task closure with complete auditability and zero hallucinated progress.

### ADR-002: Two-Stage System 1 Routing (Deterministic Patterns vs Oracle Fallback)
- **Status:** Accepted
- **Context:** Routing every coding task to an expensive, unconstrained frontier LLM wastes massive compute on standard architectural boilerplate (Axum handlers, CLI subcommands, CRUD services).
- **Decision:** Implement a two-stage router: (1) evaluate task intent and AST signatures against the `dado` pattern catalog; if matched, generate structural code deterministically with constrained hole filling; (2) if no pattern matches, escalate to frontier Oracle synthesis.
- **Consequences:** 85%+ reduction in token spend on routine code, reserving expensive frontier reasoning for genuinely novel algorithms.

### ADR-003: Progressive Self-Repair Loop with Finite Ceilings (3x)
- **Status:** Accepted
- **Context:** When generated code fails compilation, asking a frontier model to re-generate the entire file from scratch is slow, expensive, and risks introducing new bugs into working code.
- **Decision:** Map compiler stderr diagnostics back to specific typed hole spans. Re-prompt the constrained hole filler with localized diagnostic errors, allowing up to 3 repair iterations. If errors persist after 3 attempts, escalate to unconstrained Oracle generation.
- **Consequences:** Fast, low-cost error repair that fixes 75%+ of compiler warnings on attempt #2 without blowing token budgets.

### ADR-004: Ephemeral Git Worktree Sandboxing for Side-Effect-Free Synthesis
- **Status:** Accepted
- **Context:** Running synthesis and speculative compilation directly on the developer's working directory risks leaving behind modified files, uncommitted changes, or broken builds when runs fail.
- **Decision:** Support `--sandbox` mode using `miter-harness`: spawn an ephemeral detached git worktree (`git worktree add --detach`), apply patches, compile, and run tests. Only promote changes to the working tree upon verified success.
- **Consequences:** Complete isolation with zero working tree pollution during failed or interrupted synthesis attempts.

### ADR-005: Outbox Spool Telemetry with AST Secret Redaction
- **Status:** Accepted
- **Context:** Training specialized open-weight code models (`rabbet`) requires rich synthesis traces, but sending raw developer code or credentials creates serious privacy and data security violations.
- **Decision:** Decouple telemetry via `.gaugeblock/spool/outbox.jsonl`. Pass all traces through `miter` secret scrubbing and variable anonymization (`sym_fn_1`, `sym_var_2`) before writing to the outbox.
- **Consequences:** Safe, privacy-preserving continuous model improvement without exposing proprietary business logic or API keys.

# Product Vision & Specifications — `joiner`

> **Domain:** Master Neurosymbolic Synthesis Orchestrator  
> **Ecosystem:** `gaugeblock` Neurosymbolic Meta-Harness  
> **License:** Apache-2.0

---

## 1. Problem Statement & Market Context

Current autonomous coding agents operate as open-loop text generators: they emit multi-file code diffs, cross their fingers, and report that a task is done without verifying whether the codebase compiles or passes tests. This creates four acute pain points:

1. **Hallucinated Task Completion:** Agents mark backlog tickets "done" when the code contains hidden compilation errors, syntax typos, or failing assertions.
2. **Expensive Token Inefficiency:** Asking models to generate 100% of code from scratch burns 85%+ of compute and context on predictable framework syntax and boilerplate.
3. **Flawed Self-Repair Cascades:** When a build breaks, agents re-generate entire files from scratch, frequently fixing one error while introducing three new ones.
4. **Wasted Execution Telemetry:** Thousands of hours of agent code generation evaporate into lost chat logs rather than being captured as compiler-verified training data to train smaller, faster models.

`joiner` solves this by orchestrating a closed neurosymbolic synthesis and verification loop.

---

## 2. Target Personas & User Journeys

### Persona A: Autonomous AI Software Engineer
- **Needs:** Closed-loop execution engine that leases tasks from `storypole`, coordinates structural templates (`dado`), generates constrained logic (`sawhorse` + `batten`), and certifies verification.
- **Workflow:** Runs `joiner run-next --sandbox` to autonomously execute the highest-priority task in an isolated worktree and commit verified code upon completion.

### Persona B: Systems Lead & Platform Architect
- **Needs:** Guaranteed verification standards; zero regressions; complete auditability linking tasks to verified commit SHAs.
- **Workflow:** Inspects synthesis events live in `benchwork`; verifies that no PR merges without an immutable `VerificationReceipt`.

### Persona C: Open-Weight Model Researcher (`rabbet`)
- **Needs:** Large streams of compiler-verified, AST-anonymized synthesis traces (Bronze/Silver/Gold) for supervised fine-tuning (SFT) and Direct Preference Optimization (DPO).
- **Workflow:** Ingests `joiner`'s `.gaugeblock/spool/outbox.jsonl` traces into `rabbet` to continuously fine-tune specialized 7B micro-models.

---

## 3. Jobs to Be Done (JTBD)

| When I... | I want to... | So that I can... |
|---|---|---|
| Assign a task to an autonomous agent | Pull the next ready task from `storypole` and synthesize it in a sandboxed worktree | Execute code modifications without polluting my checked-out working directory on failure. |
| Generate code for a standard architectural pattern | Route the task to a verified `dado` pattern and fill only the typed holes with grammar masks | Eliminate 85%+ of LLM token spend and avoid syntax errors in boilerplate code. |
| Encounter compiler warnings or errors during synthesis | Map the diagnostics back to exact hole spans and re-prompt the filler | Self-repair code quickly without re-generating the entire file from scratch. |
| Complete a synthesis run | Record compiler exit codes, commit SHAs, and anonymized traces in the outbox spool | Provide proof of correctness for CI while feeding continuous telemetry into the open-weight training foundry. |

---

## 4. Key Metrics & Success Criteria

* **Verification Precision:** `100%` of tasks closed by `joiner` pass compiler checks and test suites.
* **Token Budget Reduction:** `>85%` reduction in token consumption compared to whole-file generation.
* **Self-Repair Success Rate:** `>75%` of initial compiler diagnostics resolved on iteration #1 or #2.
* **Zero Telemetry Leaks:** `0` raw credentials or proprietary identifiers emitted to the spool outbox.

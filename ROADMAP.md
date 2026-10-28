# Product Roadmap & Strategic Horizons — `joiner`

> **Active Suite Lifecycle:** `0.1.0-beta.X`  
> **Target:** Master Neurosymbolic Synthesis Orchestrator

---

## 1. Active Milestone: `v0.1.0-beta.3` (Current Focus)

- [x] **Closed-Loop Synthesis Pipeline:** End-to-end integration across `storypole`, `dado`, `batten`, `sawhorse`, and `miter`.
- [x] **Two-Stage System 1 Router:** Deterministic pattern lookup with frontier Oracle fallback.
- [x] **Parallel Hole Filling:** Concurrent completion of independent typed holes with sequential fallback.
- [x] **Progressive Self-Repair Loop:** Compiler diagnostic attribution to exact hole spans with 3x repair ceiling.
- [x] **Ephemeral Git Worktree Sandboxing:** Side-effect-free compiler and test verification.
- [x] **Telemetry Spool Outbox:** Privacy-scrubbed JSONL outbox (`.gaugeblock/spool/`) for `rabbet` fine-tuning.
- [x] **Axum HTTP/WebSocket Daemon:** Live event streaming on port 7680 for `benchwork` visualization.
- [x] **9-Document Architecture Compliance:** Certified clean under `chalkline docs check`.

---

## 2. Horizon 1: Inter-Hole Dependencies & Speculative Execution (`v0.2.0`)

- [ ] **Inter-Hole Dependency DAG:** Resolve dependencies between typed holes (e.g. return type of Hole A feeding argument of Hole B).
- [ ] **Speculative Multi-Candidate Sampling:** Concurrently sample 3 candidate fills from local micro-models and rank by pre-flight validation.
- [ ] **Automated DPO Pair Extraction:** Automatically convert self-repaired failures into Gold DPO preference pairs for `rabbet`.
- [ ] **TypeScript / Node.js Compiler Backend:** Dedicated verification backend parsing `tsc` and `vitest` diagnostics in sandboxes.

---

## 3. Horizon 2: Continuous Background Daemon & Live TUI (`v0.3.0`)

- [ ] **Continuous Synthesis Daemon:** Standalone background service that continuously claims unblocked ready tasks from `storypole`.
- [ ] **Interactive Terminal Dashboard:** Ratatui-powered terminal monitor tracking live synthesis passes, hole fills, and compiler outputs.
- [ ] **Dynamic Strategy Tuner:** Machine learning routing adjusting threshold between `dado` patterns and Oracle based on historical success rates.

---

## 4. Horizon 3: Swarm Synthesis & Enterprise Verification (`v1.0.0`)

- [ ] **Distributed Swarm Orchestration:** Coordinate 10+ concurrent agent workers across multi-repository workspaces.
- [ ] **Enterprise Verification Export:** Generate cryptographically signed compliance attestations for mission-critical deployments.
- [ ] **Long-Term Wire Stability:** Freeze synthesis event schemas and telemetry formats for multi-year backward compatibility.

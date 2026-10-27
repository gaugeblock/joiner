# Behavioral Specifications & Invariants

## Formal Type Contracts

- **`model::AnonymizedSynthesisTrace`** (`struct`): Wire contract representing an AST-anonymized synthesis run per `CONTRACTS.md`.
- **`mod::AppState`** (`struct`): Shared application state for all daemon handlers.
- **`verifier::AstVerifier`** (`struct`): Verification engine coordinating splicing, AST validation, compiler checks, and tests.
- **`args::CliArgs`** (`struct`): Public interface entity
- **`args::Command`** (`enum`): Commands supported by the `joiner` CLI.
- **`model::CompilerDiagnostic`** (`struct`): Located compiler diagnostic reported during verification.
- **`args::DaemonArgs`** (`struct`): Arguments for `joiner daemon`.
- **`model::ExecutionMode`** (`enum`): Execution environment mode for synthesis and verification.
- **`spool::FileSpoolSink`** (`struct`): Local JSONL file-backed telemetry outbox for rabbet synchronization.
- **`filler::HoleFillRequest`** (`struct`): Context parameters driving hole generation or localized repair.
- **`filler::HoleFiller`** (`struct`): Orchestrates batten grammar compilation, LLM sampling, and candidate validation.
- **`model::JoinerConfig`** (`struct`): Global orchestration configuration for `JoinerEngine`.
- **`mod::JoinerEngine`** (`struct`): Master neurosymbolic synthesis engine.
- **`error::JoinerError`** (`enum`): Primary error enumeration covering all orchestration phases in `joiner`.
- **`sink::MemoryTelemetrySink`** (`struct`): In-memory sink implementation for testing and ephemeral execution.
- **`oracle::OracleResult`** (`struct`): Result of an Oracle synthesis attempt.
- **`oracle::OracleRunner`** (`struct`): Unconstrained frontier model runner for novel synthesis and terminal self-repair.
- **`args::RunNextArgs`** (`struct`): Arguments for `joiner run-next`.
- **`sink::SpoolEntry`** (`struct`): A serialized telemetry entry tracked in the local outbox.
- **`sink::SyncStatus`** (`enum`): Synchronization status of a telemetry trace in the outbox.

//! Master JoinerEngine orchestration loop coordinating the neurosymbolic suite.

pub mod synthesis;

use crate::error::{JoinerError, Result};
use crate::filler::HoleFiller;
use crate::model::{JoinerConfig, SynthesisEvent, SynthesisResult};
use crate::oracle::OracleRunner;
use crate::router::System1Router;
use crate::telemetry::TelemetrySink;
use crate::verifier::AstVerifier;
use dado_core::PatternRegistry;
use sawhorse_core::router::RoleRouter;
use std::sync::Arc;
use storypole_core::index::query::TaskFilter;
use storypole_core::model::decision::VerificationReceipt;
use storypole_core::model::task::TaskNode;
use storypole_core::ops::workflow::{next_task, verify_task};
use storypole_core::store::StorypoleStore;
use tokio::sync::broadcast;
use tracing::info;

/// Master neurosymbolic synthesis engine.
#[derive(Debug, Clone)]
pub struct JoinerEngine {
    config: JoinerConfig,
    registry: Arc<PatternRegistry>,
    router: System1Router,
    filler: HoleFiller,
    verifier: AstVerifier,
    oracle: OracleRunner,
    telemetry_sink: Arc<dyn TelemetrySink>,
    event_tx: broadcast::Sender<SynthesisEvent>,
}

impl JoinerEngine {
    /// Initializes a new `JoinerEngine`.
    pub fn new(
        config: JoinerConfig,
        registry: Arc<PatternRegistry>,
        role_router: Arc<RoleRouter>,
        telemetry_sink: Arc<dyn TelemetrySink>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(128);
        let router = System1Router::new(registry.clone(), Some(role_router.clone()));
        let filler = HoleFiller::new(role_router.clone());
        let verifier = AstVerifier;
        let oracle = OracleRunner::new(role_router);

        Self {
            config,
            registry,
            router,
            filler,
            verifier,
            oracle,
            telemetry_sink,
            event_tx,
        }
    }

    /// Access engine configuration.
    pub fn config(&self) -> &JoinerConfig {
        &self.config
    }

    /// Subscribes to real-time synthesis lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<SynthesisEvent> {
        self.event_tx.subscribe()
    }

    /// Emits a lifecycle event to all active subscribers.
    pub fn emit(&self, event: SynthesisEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Claims and executes the next ready task from `storypole`.
    pub async fn run_next(&self) -> Result<SynthesisResult> {
        let mut store = StorypoleStore::open(&self.config.workspace_root)?;
        let filter = TaskFilter::default();
        let briefing = next_task(&mut store, &filter, Some(&self.config.agent_id))?
            .ok_or(JoinerError::NoReadyTask)?;

        let task = briefing.task;
        info!(ticket = %task.ticket_key, title = %task.title, "Claimed ready task");
        self.emit(SynthesisEvent::TaskClaimed {
            task_id: task.id,
            ticket_key: task.ticket_key.clone(),
        });

        let res = self.synthesize_task(&task).await?;

        let receipt = VerificationReceipt {
            verifier: "joiner".into(),
            command: "joiner synthesize".into(),
            exit_code: Some(0),
            diagnostic_count: Some(0),
            summary: format!(
                "Synthesized and verified via {:?} (Tier: {:?}, Trace: {})",
                res.strategy, res.tier, res.task_id
            ),
            verified_at_epoch_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| JoinerError::SpoolError(e.to_string()))?
                .as_millis() as u64,
            git_branch: None,
            base_commit_sha: None,
            head_commit_sha: None,
            commit_shas: Vec::new(),
        };

        verify_task(&mut store, task.id, Some(receipt), false, false)?;
        Ok(res)
    }

    /// Synthesizes and verifies code for a specific `TaskNode`.
    pub async fn synthesize_task(&self, task: &TaskNode) -> Result<SynthesisResult> {
        synthesis::execute_task_synthesis(
            &self.config,
            &self.registry,
            &self.router,
            &self.filler,
            &self.verifier,
            &self.oracle,
            &self.telemetry_sink,
            &self.event_tx,
            task,
        )
        .await
    }
}


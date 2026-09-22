use async_trait::async_trait;
use std::sync::Arc;
use talos_core::TalosError;
use talos_types::IntakeEnvelope;
use tokio::sync::RwLock;

#[async_trait]
pub trait FrameSink: Send + Sync {
    async fn submit(&self, envelope: IntakeEnvelope) -> Result<(), TalosError>;
}

/// In-memory sink for tests and local CLI.
#[derive(Clone, Default)]
pub struct InMemoryFrameSink {
    inner: Arc<RwLock<Vec<IntakeEnvelope>>>,
}

impl InMemoryFrameSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn envelopes(&self) -> Vec<IntakeEnvelope> {
        self.inner.read().await.clone()
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }
}

#[async_trait]
impl FrameSink for InMemoryFrameSink {
    async fn submit(&self, envelope: IntakeEnvelope) -> Result<(), TalosError> {
        self.inner.write().await.push(envelope);
        Ok(())
    }
}

/// Sink that always fails (tests FailedSink / no silent drop).
#[derive(Clone, Default)]
pub struct FailingFrameSink;

#[async_trait]
impl FrameSink for FailingFrameSink {
    async fn submit(&self, _envelope: IntakeEnvelope) -> Result<(), TalosError> {
        Err(TalosError::Transient("sink unavailable".into()))
    }
}

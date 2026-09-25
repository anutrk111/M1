use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use talos_core::TalosError;
use talos_types::{FrameId, IntakeEnvelope};
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

/// Durable-sink idempotency key: `FrameId` + SHA-256 of the **original** bytes.
///
/// `FrameId` is a random ULID minted per frame, so the key is only stable across
/// retries when the caller reuses the `FrameId` (recovery re-runs do this for
/// `FailedSink` frames; see [`RecoveryContext`](crate::RecoveryContext)).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey {
    pub frame_id: FrameId,
    pub sha256: String,
}

impl IdempotencyKey {
    pub fn new(frame_id: FrameId, sha256: impl Into<String>) -> Self {
        Self {
            frame_id,
            sha256: sha256.into(),
        }
    }

    pub fn for_envelope(envelope: &IntakeEnvelope) -> Self {
        Self::new(envelope.frame_id.clone(), envelope.image.sha256.clone())
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.frame_id, self.sha256)
    }
}

/// Positive acknowledgement from a [`DurableSink`].
///
/// Negative acknowledgement (nack) is `Err(TalosError)`:
/// - `Transient` — retryable (broker down, timeout, backpressure);
/// - `Permanent` / `Validation` — poison message, do not retry as-is;
/// - `NotImplemented` — no durable backend configured.
///
/// Every nack maps to `IntakeStatus::FailedSink` in the manifest (never a silent drop).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SinkAck {
    /// Newly and durably stored.
    Accepted,
    /// The key was already durably stored (e.g. retry after a lost ack); nothing stored twice.
    Duplicate,
}

/// Durable hand-off boundary with explicit ack/nack and idempotency.
///
/// Implementations must be idempotent on `key`: offering an already-stored key returns
/// `Ok(SinkAck::Duplicate)` without storing a second copy. Real brokers are M10.
#[async_trait]
pub trait DurableSink: Send + Sync {
    async fn offer(
        &self,
        envelope: IntakeEnvelope,
        key: IdempotencyKey,
    ) -> Result<SinkAck, TalosError>;
}

#[derive(Default)]
struct DurableState {
    keys: HashSet<IdempotencyKey>,
    envelopes: Vec<IntakeEnvelope>,
}

/// In-memory [`DurableSink`] that dedupes by [`IdempotencyKey`] (tests / local CLI).
#[derive(Clone, Default)]
pub struct InMemoryDurableSink {
    inner: Arc<RwLock<DurableState>>,
}

impl InMemoryDurableSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn envelopes(&self) -> Vec<IntakeEnvelope> {
        self.inner.read().await.envelopes.clone()
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.envelopes.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.envelopes.is_empty()
    }

    pub async fn contains(&self, key: &IdempotencyKey) -> bool {
        self.inner.read().await.keys.contains(key)
    }
}

#[async_trait]
impl DurableSink for InMemoryDurableSink {
    async fn offer(
        &self,
        envelope: IntakeEnvelope,
        key: IdempotencyKey,
    ) -> Result<SinkAck, TalosError> {
        if key != IdempotencyKey::for_envelope(&envelope) {
            return Err(TalosError::Validation(format!(
                "idempotency key {key} does not match envelope {}",
                IdempotencyKey::for_envelope(&envelope)
            )));
        }
        let mut state = self.inner.write().await;
        if !state.keys.insert(key) {
            return Ok(SinkAck::Duplicate);
        }
        state.envelopes.push(envelope);
        Ok(SinkAck::Accepted)
    }
}

/// Placeholder durable sink: always nacks with `NotImplemented` (no broker until M10).
#[derive(Clone, Copy, Debug, Default)]
pub struct UnconfiguredDurableSink;

#[async_trait]
impl DurableSink for UnconfiguredDurableSink {
    async fn offer(
        &self,
        _envelope: IntakeEnvelope,
        _key: IdempotencyKey,
    ) -> Result<SinkAck, TalosError> {
        Err(TalosError::NotImplemented(
            "durable sink backend not configured (M10)".into(),
        ))
    }
}

/// Lets the intake pipeline drive a [`DurableSink`] through the [`FrameSink`] interface.
///
/// - `Ok(Accepted)` / `Ok(Duplicate)` → `Ok(())` → frame `Accepted` (stored exactly once).
/// - `Err(nack)` → `Err(nack)` → frame `FailedSink`, batch `PartialFailure`
///   (or batch `Err` when `fail_batch_on_sink_errors = true`).
pub struct DurableSinkAdapter<D: DurableSink> {
    inner: Arc<D>,
    duplicate_acks: AtomicU32,
}

impl<D: DurableSink> DurableSinkAdapter<D> {
    pub fn new(inner: Arc<D>) -> Self {
        Self {
            inner,
            duplicate_acks: AtomicU32::new(0),
        }
    }

    pub fn inner(&self) -> &Arc<D> {
        &self.inner
    }

    /// Number of `SinkAck::Duplicate` acks observed through this adapter.
    pub fn duplicate_acks(&self) -> u32 {
        self.duplicate_acks.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl<D: DurableSink> FrameSink for DurableSinkAdapter<D> {
    async fn submit(&self, envelope: IntakeEnvelope) -> Result<(), TalosError> {
        let key = IdempotencyKey::for_envelope(&envelope);
        match self.inner.offer(envelope, key).await? {
            SinkAck::Accepted => Ok(()),
            SinkAck::Duplicate => {
                self.duplicate_acks.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use talos_types::{BatchId, ImageRef, SourceKind, SourceRef};

    fn envelope(frame: &str, sha: &str) -> IntakeEnvelope {
        IntakeEnvelope::new(
            BatchId::new("b1"),
            FrameId::new(frame),
            SourceRef {
                kind: SourceKind::Upload,
                path_or_key: "x.jpg".into(),
            },
            ImageRef {
                content_type: "image/jpeg".into(),
                sha256: sha.into(),
                bytes_ref: "file:///tmp/x.jpg".into(),
            },
        )
    }

    #[tokio::test]
    async fn in_memory_durable_sink_dedupes_by_key() {
        let sink = InMemoryDurableSink::new();
        let env = envelope("f1", "abc");
        let key = IdempotencyKey::for_envelope(&env);
        assert_eq!(
            sink.offer(env.clone(), key.clone()).await.unwrap(),
            SinkAck::Accepted
        );
        assert_eq!(
            sink.offer(env.clone(), key.clone()).await.unwrap(),
            SinkAck::Duplicate
        );
        assert_eq!(sink.len().await, 1);
        assert!(sink.contains(&key).await);

        let other = envelope("f2", "abc");
        let other_key = IdempotencyKey::for_envelope(&other);
        assert_eq!(
            sink.offer(other, other_key).await.unwrap(),
            SinkAck::Accepted
        );
        assert_eq!(sink.len().await, 2);
    }

    #[tokio::test]
    async fn mismatched_key_is_validation_nack() {
        let sink = InMemoryDurableSink::new();
        let env = envelope("f1", "abc");
        let err = sink
            .offer(env, IdempotencyKey::new(FrameId::new("f1"), "zzz"))
            .await
            .unwrap_err();
        assert!(matches!(err, TalosError::Validation(_)));
        assert!(sink.is_empty().await);
    }

    #[tokio::test]
    async fn unconfigured_durable_sink_is_not_implemented() {
        let env = envelope("f1", "abc");
        let key = IdempotencyKey::for_envelope(&env);
        let err = UnconfiguredDurableSink.offer(env, key).await.unwrap_err();
        assert!(matches!(err, TalosError::NotImplemented(_)));
    }

    #[tokio::test]
    async fn adapter_counts_duplicate_acks() {
        let inner = Arc::new(InMemoryDurableSink::new());
        let adapter = DurableSinkAdapter::new(inner.clone());
        let env = envelope("f1", "abc");
        adapter.submit(env.clone()).await.unwrap();
        adapter.submit(env).await.unwrap();
        assert_eq!(adapter.duplicate_acks(), 1);
        assert_eq!(inner.len().await, 1);
    }
}

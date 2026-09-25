//! Batch recovery / idempotent re-run.
//!
//! `FrameId` is a random ULID (`talos_core::util::new_frame_id`), so it cannot identify
//! "the same file" across runs. Recovery therefore keys on `(relative_path, sha256)`:
//! - already accepted keys are skipped (`SkippedAlreadyAccepted`, not re-emitted);
//! - prior `FailedSink` keys are retried **with their original `FrameId`**, so the
//!   durable-sink [`IdempotencyKey`](crate::IdempotencyKey) (`FrameId` + sha256) is
//!   stable and a lost-ack retry dedupes to `SinkAck::Duplicate`.

use crate::types::{BatchManifest, IntakeStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use talos_types::{BatchId, FrameId, SourceKind};

/// Identity of a staged candidate across runs of the same batch.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecoveryKey {
    pub relative_path: String,
    pub sha256: String,
}

impl RecoveryKey {
    pub fn new(relative_path: impl Into<String>, sha256: impl Into<String>) -> Self {
        Self {
            relative_path: relative_path.into(),
            sha256: sha256.into(),
        }
    }
}

/// Prior-run state used to re-run intake for an existing batch.
#[derive(Clone, Debug, PartialEq)]
pub struct RecoveryContext {
    batch_id: BatchId,
    source_kind: SourceKind,
    accepted: HashMap<RecoveryKey, FrameId>,
    retry: HashMap<RecoveryKey, FrameId>,
}

impl RecoveryContext {
    /// Empty context for `batch_id`; add keys with [`with_accepted`](Self::with_accepted)
    /// / [`with_retry`](Self::with_retry).
    pub fn new(batch_id: BatchId, source_kind: SourceKind) -> Self {
        Self {
            batch_id,
            source_kind,
            accepted: HashMap::new(),
            retry: HashMap::new(),
        }
    }

    /// Build from a prior manifest: `Accepted` and `SkippedAlreadyAccepted` frames become
    /// already-accepted keys; `FailedSink` frames become retry keys (FrameId reused).
    pub fn from_manifest(prior: &BatchManifest) -> Self {
        let mut ctx = Self::new(prior.batch_id.clone(), prior.source_summary.kind.clone());
        for frame in &prior.frames {
            let (Some(frame_id), Some(sha)) = (&frame.frame_id, &frame.sha256) else {
                continue;
            };
            let key = RecoveryKey::new(frame.relative_path.clone(), sha.clone());
            match frame.intake_status {
                IntakeStatus::Accepted | IntakeStatus::SkippedAlreadyAccepted => {
                    ctx.accepted.insert(key, frame_id.clone());
                }
                IntakeStatus::FailedSink => {
                    ctx.retry.insert(key, frame_id.clone());
                }
                IntakeStatus::RejectedValidation
                | IntakeStatus::RejectedPermanent
                | IntakeStatus::SkippedDuplicate => {}
            }
        }
        ctx
    }

    pub fn with_accepted(mut self, key: RecoveryKey, frame_id: FrameId) -> Self {
        self.retry.remove(&key);
        self.accepted.insert(key, frame_id);
        self
    }

    pub fn with_retry(mut self, key: RecoveryKey, frame_id: FrameId) -> Self {
        if !self.accepted.contains_key(&key) {
            self.retry.insert(key, frame_id);
        }
        self
    }

    pub fn batch_id(&self) -> &BatchId {
        &self.batch_id
    }

    pub fn source_kind(&self) -> &SourceKind {
        &self.source_kind
    }

    pub fn accepted_len(&self) -> usize {
        self.accepted.len()
    }

    pub(crate) fn accepted_frame(&self, key: &RecoveryKey) -> Option<&FrameId> {
        self.accepted.get(key)
    }

    pub(crate) fn retry_frame(&self, key: &RecoveryKey) -> Option<&FrameId> {
        self.retry.get(key)
    }

    /// First accepted `FrameId` per sha256 (deterministic: smallest relative path), used to
    /// classify a same-content file at a new path as `SkippedDuplicate`.
    pub(crate) fn accepted_by_sha(&self) -> HashMap<String, FrameId> {
        let mut keys: Vec<_> = self.accepted.iter().collect();
        keys.sort_by(|a, b| a.0.relative_path.cmp(&b.0.relative_path));
        let mut out = HashMap::new();
        for (key, frame_id) in keys {
            out.entry(key.sha256.clone())
                .or_insert_with(|| frame_id.clone());
        }
        out
    }
}

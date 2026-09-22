use serde::{Deserialize, Serialize};
use talos_types::{FrameId, IntakeEnvelope, SourceKind, SourceRef};

/// Per-frame intake disposition (manifest authority).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum IntakeStatus {
    Accepted,
    RejectedValidation,
    RejectedPermanent,
    SkippedDuplicate,
    FailedSink,
}

/// Batch-level outcome after intake finishes (or rejects the batch).
///
/// Semantics:
/// - [`Complete`](BatchOutcome::Complete): `failed_sink == 0`, no rejected frames,
///   `reconcile_ok`, and either `accepted >= 1` or an empty batch is allowed
///   (`fail_on_empty_batch = false` with no frame failures).
/// - [`PartialFailure`](BatchOutcome::PartialFailure): batch finished processing and
///   returned `Ok`, but `failed_sink > 0` and/or frame-level rejections occurred
///   (default continue policy when `fail_batch_on_sink_errors = false`).
/// - [`Rejected`](BatchOutcome::Rejected): reserved for an `Ok` with zero accepted
///   when callers choose that shape. Prefer returning `Err(TalosError)` for empty
///   batches when `fail_on_empty_batch = true` (current default).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum BatchOutcome {
    Complete,
    PartialFailure,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateReference {
    pub original_frame_id: FrameId,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<FrameId>,
    pub relative_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    pub intake_status: IntakeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<DuplicateReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope: Option<IntakeEnvelope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BatchCounts {
    /// Files seen under the batch root (not directories).
    pub filesystem_entries_seen: u32,
    /// Image-extension candidates discovered (was `discovered`).
    #[serde(alias = "discovered")]
    pub image_candidates_discovered: u32,
    pub accepted: u32,
    pub rejected: u32,
    pub skipped_duplicate: u32,
    pub failed_sink: u32,
    pub metadata_warnings: u32,
    /// Unsupported / ignored non-image files counted during discovery (explicit, not silent).
    ///
    /// If a metadata sidecar (CSV/XLSX) lives **inside** the batch root and its extension
    /// is not in `allowed_extensions`, it is counted here. Sidecars **outside** the root
    /// are not walked and therefore not counted.
    pub ignored_unsupported: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BatchManifest {
    pub schema_version: String,
    pub batch_id: talos_types::BatchId,
    pub source_summary: SourceRef,
    pub counts: BatchCounts,
    pub frames: Vec<FrameRecord>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl BatchManifest {
    pub fn reconcile_ok(&self) -> bool {
        let accounted = self.counts.accepted
            + self.counts.rejected
            + self.counts.skipped_duplicate
            + self.counts.failed_sink;
        self.counts.image_candidates_discovered == accounted
            && self.counts.filesystem_entries_seen
                == self
                    .counts
                    .image_candidates_discovered
                    .saturating_add(self.counts.ignored_unsupported)
    }
}

/// One metadata sidecar row after parse (join key separate from FrameMetadata fields).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataRecord {
    pub join_path: String,
    pub basename: String,
    pub camera_id: Option<String>,
    pub captured_at: Option<String>,
    pub location: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ImportRequest {
    pub source_kind: SourceKind,
    pub path_or_key: String,
    /// Optional CSV/XLSX sidecar path.
    pub metadata_path: Option<std::path::PathBuf>,
}

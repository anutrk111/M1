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
    pub discovered: u32,
    pub accepted: u32,
    pub rejected: u32,
    pub skipped_duplicate: u32,
    pub failed_sink: u32,
    pub metadata_warnings: u32,
    /// Unsupported / ignored non-image files counted during discovery (explicit, not silent).
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
        // discovered = image candidates processed (not ignored_unsupported)
        self.counts.discovered == accounted
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

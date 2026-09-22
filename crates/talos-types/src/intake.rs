use crate::ids::{BatchId, CameraId, FrameId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// How a frame entered the system (batch / file — not live camera).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Folder,
    Zip,
    Upload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    pub kind: SourceKind,
    pub path_or_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageRef {
    pub content_type: String,
    pub sha256: String,
    /// `file://…`, `object://…`, or other opaque reference.
    pub bytes_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FrameMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_id: Option<CameraId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Intake envelope (JSON schema v2.0). Produced by M02; defined here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntakeEnvelope {
    pub schema_version: String,
    pub batch_id: BatchId,
    pub frame_id: FrameId,
    pub source: SourceRef,
    pub image: ImageRef,
    #[serde(default)]
    pub metadata: FrameMetadata,
}

impl IntakeEnvelope {
    pub fn new(
        batch_id: BatchId,
        frame_id: FrameId,
        source: SourceRef,
        image: ImageRef,
    ) -> Self {
        Self {
            schema_version: crate::SCHEMA_VERSION.to_owned(),
            batch_id,
            frame_id,
            source,
            image,
            metadata: FrameMetadata::default(),
        }
    }
}

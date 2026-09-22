use serde::{Deserialize, Serialize};

/// Stage 0 quality gate (M04). Soft `Degraded` does not alone halt OCR/HSRP.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ImageQualityGate {
    Accept,
    Degraded,
    Unusable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageQualityReport {
    /// Overall quality score in \[0.0, 1.0\].
    pub score: f32,
    pub blur_score: f32,
    pub exposure_ok: bool,
    pub notes: Vec<String>,
    /// Optional gate for M04 Stage 0; absent in legacy fixture payloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<ImageQualityGate>,
}

/// Provider / model provenance (optional on detections and OCR).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Generic detection skeleton. M05 may specialize into plate/vehicle structs later.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    pub label: String,
    pub score: f32,
    pub bbox: BoundingBox,
    /// Talos-owned lineage id (M05+). Optional for fixture/backward compat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_id: Option<crate::ids::DetectionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderRef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DetectionSummary {
    pub raw_plate_count: u32,
    pub accepted_plate_count: u32,
    pub filtered_low_score: u32,
    pub suppressed_overlap: u32,
    pub truncated: u32,
    pub raw_vehicle_count: u32,
    pub accepted_vehicle_count: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Detections {
    pub vehicles: Vec<Detection>,
    pub plates: Vec<Detection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<DetectionSummary>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RectifiedPlate {
    pub bbox: BoundingBox,
    pub crop_ref: String,
    pub width: u32,
    pub height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_id: Option<crate::ids::DetectionId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OcrHypothesis {
    pub text: String,
    pub confidence: f32,
    #[serde(default)]
    pub char_confidences: Vec<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_id: Option<crate::ids::DetectionId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrammarResult {
    pub normalized_text: String,
    pub ok: bool,
    pub errors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_id: Option<crate::ids::DetectionId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct HsrpEvidence {
    pub score: f32,
    pub ind_mark_detected: bool,
    pub hologram_cues: bool,
    pub geometry_ok: bool,
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_id: Option<crate::ids::DetectionId>,
}

/// Stage 6 observation-cluster / dedup skeleton (not vehicle tracking).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DedupResult {
    pub is_duplicate: bool,
    pub matched_frame_id: Option<String>,
    pub method: Option<String>,
}

/// Machine decision produced by Stage 7 (immutable under M09 review).
pub type MachineDecision = FusedDecision;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedDecision {
    pub plate_text: String,
    pub grammar_ok: bool,
    pub hsrp_score: f32,
    pub fused_confidence: f32,
    pub outcome: crate::decision::DecisionOutcome,
    pub hard_fail: bool,
    pub hard_fail_reasons: Vec<String>,
    /// Config revision that governed thresholds (M12); optional until wired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_revision_id: Option<crate::ids::ConfigRevisionId>,
}

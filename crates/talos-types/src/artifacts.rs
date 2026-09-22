use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageQualityReport {
    /// Overall quality score in \[0.0, 1.0\].
    pub score: f32,
    pub blur_score: f32,
    pub exposure_ok: bool,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    pub label: String,
    pub score: f32,
    pub bbox: BoundingBox,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Detections {
    pub vehicles: Vec<Detection>,
    pub plates: Vec<Detection>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RectifiedPlate {
    pub bbox: BoundingBox,
    pub crop_ref: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OcrHypothesis {
    pub text: String,
    pub confidence: f32,
    #[serde(default)]
    pub char_confidences: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrammarResult {
    pub normalized_text: String,
    pub ok: bool,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct HsrpEvidence {
    pub score: f32,
    pub ind_mark_detected: bool,
    pub hologram_cues: bool,
    pub geometry_ok: bool,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DedupResult {
    pub is_duplicate: bool,
    pub matched_frame_id: Option<String>,
    pub method: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedDecision {
    pub plate_text: String,
    pub grammar_ok: bool,
    pub hsrp_score: f32,
    pub fused_confidence: f32,
    pub outcome: crate::decision::DecisionOutcome,
    pub hard_fail: bool,
    pub hard_fail_reasons: Vec<String>,
}

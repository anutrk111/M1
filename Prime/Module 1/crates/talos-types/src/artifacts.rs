use crate::error::ContractError;
use crate::ids::DetectionId;
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

/// Canonical bounding box: normalized, resolution-independent coordinates in
/// `[0.0, 1.0]` relative to the original frame (ADR-0041).
///
/// Invariants: all values finite, within `[0, 1]`, `x_min < x_max`, `y_min < y_max`.
/// Deserialization enforces the invariants.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "BoundingBoxWire")]
pub struct BoundingBox {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

#[derive(Deserialize)]
struct BoundingBoxWire {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
}

impl TryFrom<BoundingBoxWire> for BoundingBox {
    type Error = ContractError;
    fn try_from(w: BoundingBoxWire) -> Result<Self, Self::Error> {
        BoundingBox::new(w.x_min, w.y_min, w.x_max, w.y_max)
    }
}

/// Pixel rectangle derived from a [`BoundingBox`] at the crop/render boundary only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl BoundingBox {
    pub fn new(x_min: f32, y_min: f32, x_max: f32, y_max: f32) -> Result<Self, ContractError> {
        let b = Self {
            x_min,
            y_min,
            x_max,
            y_max,
        };
        b.validate()?;
        Ok(b)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        for (name, v) in [
            ("x_min", self.x_min),
            ("y_min", self.y_min),
            ("x_max", self.x_max),
            ("y_max", self.y_max),
        ] {
            if !v.is_finite() || !(0.0..=1.0).contains(&v) {
                return Err(ContractError::new(format!(
                    "bbox {name}={v} outside normalized [0,1]"
                )));
            }
        }
        if self.x_min >= self.x_max || self.y_min >= self.y_max {
            return Err(ContractError::new("bbox requires min < max on both axes"));
        }
        Ok(())
    }

    /// Normalize a pixel rectangle against the frame size.
    pub fn from_pixels(
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        frame_width: u32,
        frame_height: u32,
    ) -> Result<Self, ContractError> {
        if frame_width == 0 || frame_height == 0 {
            return Err(ContractError::new("frame dimensions must be non-zero"));
        }
        let fw = frame_width as f64;
        let fh = frame_height as f64;
        let x2 = x as f64 + w as f64;
        let y2 = y as f64 + h as f64;
        if x2 > fw || y2 > fh {
            return Err(ContractError::new("pixel rect exceeds frame bounds"));
        }
        Self::new(
            (x as f64 / fw) as f32,
            (y as f64 / fh) as f32,
            (x2 / fw) as f32,
            (y2 / fh) as f32,
        )
    }

    /// Derive a pixel rectangle for a frame of the given size. The rect is
    /// expanded outward to whole pixels, clamped to the frame, and is at least 1x1.
    pub fn to_pixel_rect(
        &self,
        frame_width: u32,
        frame_height: u32,
    ) -> Result<PixelRect, ContractError> {
        self.validate()?;
        if frame_width == 0 || frame_height == 0 {
            return Err(ContractError::new("frame dimensions must be non-zero"));
        }
        let fw = frame_width as f64;
        let fh = frame_height as f64;
        // f32 storage cannot represent most pixel fractions exactly; values within
        // this tolerance of a whole pixel snap to it instead of expanding outward.
        const SNAP: f64 = 1e-3;
        let snap = |v: f64, round: fn(f64) -> f64| {
            let r = v.round();
            if (v - r).abs() < SNAP {
                r
            } else {
                round(v)
            }
        };
        let x0 = snap(self.x_min as f64 * fw, f64::floor).clamp(0.0, fw - 1.0) as u32;
        let y0 = snap(self.y_min as f64 * fh, f64::floor).clamp(0.0, fh - 1.0) as u32;
        let x1 = snap(self.x_max as f64 * fw, f64::ceil).clamp(0.0, fw) as u32;
        let y1 = snap(self.y_max as f64 * fh, f64::ceil).clamp(0.0, fh) as u32;
        Ok(PixelRect {
            x: x0,
            y: y0,
            w: x1.saturating_sub(x0).max(1),
            h: y1.saturating_sub(y0).max(1),
        })
    }

    pub fn width(&self) -> f32 {
        self.x_max - self.x_min
    }

    pub fn height(&self) -> f32 {
        self.y_max - self.y_min
    }

    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }
}

/// Generic detection skeleton. M05 may specialize into plate/vehicle structs later.
///
/// `detection_id` is optional here only because raw provider detections are not
/// yet minted by M05; every downstream artifact requires it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    pub label: String,
    pub score: f32,
    pub bbox: BoundingBox,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_id: Option<DetectionId>,
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
    pub detection_id: DetectionId,
    pub bbox: BoundingBox,
    pub crop_ref: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OcrHypothesis {
    pub detection_id: DetectionId,
    pub text: String,
    pub confidence: f32,
    #[serde(default)]
    pub char_confidences: Vec<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderRef>,
}

/// Deterministic grammar verdict (M06). Grammar never invents characters (ADR-0019).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrammarStatus {
    Valid,
    Ambiguous,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrammarResult {
    pub detection_id: DetectionId,
    pub normalized_text: String,
    pub status: GrammarStatus,
    pub errors: Vec<String>,
}

impl GrammarResult {
    pub fn is_valid(&self) -> bool {
        self.status == GrammarStatus::Valid
    }
}

/// Structured OCR outcome for one detection (M06).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OcrResult {
    pub detection_id: DetectionId,
    pub candidates: Vec<OcrHypothesis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<OcrHypothesis>,
    pub grammar: GrammarResult,
}

impl OcrResult {
    /// All nested artifacts must share this result's `DetectionId`.
    pub fn validate_lineage(&self) -> Result<(), ContractError> {
        let same = |id: &DetectionId| id == &self.detection_id;
        if !self.candidates.iter().all(|c| same(&c.detection_id))
            || !self.selected.iter().all(|s| same(&s.detection_id))
            || !same(&self.grammar.detection_id)
        {
            return Err(ContractError::new("OcrResult lineage mismatch"));
        }
        Ok(())
    }
}

/// Ternary cue observability (ADR-0020). `NotObservable` is never a negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CueObservation {
    Observed,
    NotObserved,
    NotObservable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HsrpEvidence {
    pub detection_id: DetectionId,
    pub score: f32,
    pub ind_mark: CueObservation,
    pub hologram: CueObservation,
    pub geometry: CueObservation,
    pub notes: Vec<String>,
}

/// Stage 6 observation-cluster / dedup skeleton (not vehicle tracking).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DedupResult {
    pub is_duplicate: bool,
    pub matched_frame_id: Option<String>,
    pub method: Option<String>,
}

/// Stage 7 fusion working state (M01 fixture / M08). Exported downstream as
/// [`crate::MachineDecision`].
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

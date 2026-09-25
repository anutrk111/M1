//! AI usage accounting contract (M03 emits, M10 persists).

use crate::ids::{BatchId, FrameId, TraceId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiOperation {
    DetectVehiclesPlates,
    RunOcr,
    AnalyzeHsrp,
    VlmAssist,
}

impl AiOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DetectVehiclesPlates => "detect_vehicles_plates",
            Self::RunOcr => "run_ocr",
            Self::AnalyzeHsrp => "analyze_hsrp",
            Self::VlmAssist => "vlm_assist",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostStatus {
    Ok,
    Error,
}

/// One billable provider attempt (success or quota-consuming failure).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CostEvent {
    pub trace_id: TraceId,
    pub frame_id: FrameId,
    pub batch_id: BatchId,
    pub provider: String,
    #[serde(default)]
    pub model: Option<String>,
    pub operation: AiOperation,
    pub status: CostStatus,
    pub latency_ms: u64,
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    pub image_units: u32,
    #[serde(default)]
    pub usd_estimate: Option<f64>,
    #[serde(default)]
    pub error_class: Option<String>,
}

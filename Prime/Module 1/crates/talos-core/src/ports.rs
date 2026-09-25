//! Hexagonal ports owned by M01. Adapters live in their modules (M03 implements
//! [`VisionGateway`]); stages depend only on these traits.

use crate::error::TalosError;
use async_trait::async_trait;
use talos_types::{
    BatchId, DetectionId, Detections, Evidence, FrameId, HsrpEvidence, OcrHypothesis, TraceId,
};

/// Image input for a remote vision call. Exactly one of `image_bytes` /
/// `bytes_ref` must be set.
#[derive(Clone, Debug)]
pub struct VisionRequest {
    pub trace_id: TraceId,
    pub frame_id: FrameId,
    pub batch_id: BatchId,
    /// Required for OCR and HSRP (lineage, ADR-0042); absent for detection.
    pub detection_id: Option<DetectionId>,
    pub image_bytes: Option<Vec<u8>>,
    pub bytes_ref: Option<String>,
    pub content_type: String,
}

impl VisionRequest {
    pub fn validate(&self) -> Result<(), TalosError> {
        match (&self.image_bytes, &self.bytes_ref) {
            (Some(_), Some(_)) | (None, None) => {
                return Err(TalosError::Validation(
                    "exactly one of image_bytes or bytes_ref must be set".into(),
                ))
            }
            (Some(b), None) if b.is_empty() => {
                return Err(TalosError::Validation("image_bytes is empty".into()))
            }
            (None, Some(r)) if r.trim().is_empty() => {
                return Err(TalosError::Validation("bytes_ref is empty".into()))
            }
            _ => {}
        }
        if !matches!(self.content_type.as_str(), "image/jpeg" | "image/png") {
            return Err(TalosError::Validation(format!(
                "unsupported content_type {}",
                self.content_type
            )));
        }
        Ok(())
    }

    pub fn require_detection_id(&self) -> Result<&DetectionId, TalosError> {
        self.detection_id
            .as_ref()
            .ok_or_else(|| TalosError::Validation("operation requires DetectionId lineage".into()))
    }
}

#[derive(Clone, Debug)]
pub struct VlmRequest {
    pub vision: VisionRequest,
    /// Why assist was requested. Carries no plate-override authority.
    pub prompt_context: String,
}

#[derive(Clone, Debug)]
pub struct VlmAssistResult {
    /// Always `EvidenceKind::VisualObservation` (ADR-0006).
    pub evidence: Evidence,
    /// Optional structured notes for M08; non-authoritative.
    pub hints: serde_json::Value,
}

/// OCR from a primary provider plus, when configured and the providers
/// disagree, a secondary and an arbiter opinion. All hypotheses are evidence;
/// none overrides M06 grammar.
#[derive(Clone, Debug, Default)]
pub struct OcrConsensus {
    pub primary: Vec<OcrHypothesis>,
    pub secondary: Option<Vec<OcrHypothesis>>,
    pub arbiter: Option<Vec<OcrHypothesis>>,
    /// `true` when secondary was consulted and agreed with primary's top text.
    pub agreed: Option<bool>,
}

impl OcrConsensus {
    /// Every hypothesis in provider order (primary, secondary, arbiter).
    pub fn all(&self) -> Vec<OcrHypothesis> {
        let mut out = self.primary.clone();
        out.extend(self.secondary.iter().flatten().cloned());
        out.extend(self.arbiter.iter().flatten().cloned());
        out
    }
}

/// Sole egress port for remote vision / VLM (M03 PDR section 6).
#[async_trait]
pub trait VisionGateway: Send + Sync {
    async fn detect_vehicles_plates(&self, req: VisionRequest) -> Result<Detections, TalosError>;

    async fn run_ocr(&self, req: VisionRequest) -> Result<Vec<OcrHypothesis>, TalosError>;

    async fn analyze_hsrp(&self, req: VisionRequest) -> Result<HsrpEvidence, TalosError>;

    async fn vlm_assist(&self, req: VlmRequest) -> Result<VlmAssistResult, TalosError>;

    /// Default: primary-only consensus.
    async fn run_ocr_consensus(&self, req: VisionRequest) -> Result<OcrConsensus, TalosError> {
        Ok(OcrConsensus {
            primary: self.run_ocr(req).await?,
            ..OcrConsensus::default()
        })
    }
}

//! Deterministic fixture Stages 0–7 for offline CI spine (no GPU / network).
//!
//! These are development helpers that satisfy M01 contracts. Production stage
//! logic belongs in M04–M08 / M03.

use crate::error::TalosError;
use crate::pipeline::{FrameContext, Pipeline, Stage, StageId, StageStatus};
use async_trait::async_trait;
use std::sync::Arc;
use talos_config::DecisionConfig;
use talos_types::*;

/// Build a full fixture pipeline using the given decision thresholds.
pub fn fixture_pipeline(decision: DecisionConfig) -> Result<Pipeline, TalosError> {
    let stages: Vec<Arc<dyn Stage>> = vec![
        Arc::new(FixtureImageQuality),
        Arc::new(FixtureDetection),
        Arc::new(FixtureRectification),
        Arc::new(FixtureOcr),
        Arc::new(FixtureGrammar),
        Arc::new(FixtureHsrp),
        Arc::new(FixtureDedup),
        Arc::new(FixtureDecision { decision }),
    ];
    Pipeline::new(stages)
}

pub struct FixtureImageQuality;
pub struct FixtureDetection;
pub struct FixtureRectification;
pub struct FixtureOcr;
pub struct FixtureGrammar;
pub struct FixtureHsrp;
pub struct FixtureDedup;
pub struct FixtureDecision {
    pub decision: DecisionConfig,
}

#[async_trait]
impl Stage for FixtureImageQuality {
    fn id(&self) -> StageId {
        StageId::ImageQuality
    }
    fn name(&self) -> &'static str {
        "fixture_image_quality"
    }
    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
        ctx.quality = Some(ImageQualityReport {
            score: 0.92,
            blur_score: 0.15,
            exposure_ok: true,
            notes: vec!["fixture".into()],
            gate: Some(ImageQualityGate::Accept),
        });
        ctx.push_visual_evidence(Evidence::visual(
            "fixture.s0",
            serde_json::json!({ "score": 0.92 }),
            0.92,
        ))?;
        Ok(StageStatus::Continue)
    }
}

#[async_trait]
impl Stage for FixtureDetection {
    fn id(&self) -> StageId {
        StageId::Detection
    }
    fn name(&self) -> &'static str {
        "fixture_detection"
    }
    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
        let plate = Detection {
            label: "plate".into(),
            score: 0.94,
            bbox: BoundingBox::new(0.10, 0.40, 0.34, 0.46)?,
            detection_id: Some(DetectionId::new("fixture-plate-1")),
            provider: Some(ProviderRef {
                name: "fixture".into(),
                model: None,
            }),
        };
        ctx.detections = Some(Detections {
            vehicles: vec![Detection {
                label: "car".into(),
                score: 0.91,
                bbox: BoundingBox::new(0.04, 0.16, 0.54, 0.88)?,
                detection_id: Some(DetectionId::new("fixture-vehicle-1")),
                provider: Some(ProviderRef {
                    name: "fixture".into(),
                    model: None,
                }),
            }],
            plates: vec![plate],
            summary: Some(DetectionSummary {
                raw_plate_count: 1,
                accepted_plate_count: 1,
                filtered_low_score: 0,
                suppressed_overlap: 0,
                truncated: 0,
                raw_vehicle_count: 1,
                accepted_vehicle_count: 1,
            }),
        });
        ctx.push_visual_evidence(Evidence::visual(
            "fixture.s1",
            serde_json::json!({ "plates": 1 }),
            0.94,
        ))?;
        Ok(StageStatus::Continue)
    }
}

#[async_trait]
impl Stage for FixtureRectification {
    fn id(&self) -> StageId {
        StageId::Rectification
    }
    fn name(&self) -> &'static str {
        "fixture_rectification"
    }
    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
        let Some(plate) = ctx.detections.as_ref().and_then(|d| d.plates.first()) else {
            return Ok(StageStatus::SkipRemaining);
        };
        let detection_id = plate.detection_id.clone().ok_or_else(|| {
            TalosError::Validation("plate detection missing DetectionId lineage".into())
        })?;
        ctx.rectified = Some(RectifiedPlate {
            detection_id,
            bbox: plate.bbox,
            crop_ref: format!("fixture://crop/{}", ctx.intake.frame_id),
            width: 240,
            height: 60,
        });
        Ok(StageStatus::Continue)
    }
}

#[async_trait]
impl Stage for FixtureOcr {
    fn id(&self) -> StageId {
        StageId::Ocr
    }
    fn name(&self) -> &'static str {
        "fixture_ocr"
    }
    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
        // Deterministic fixture plate — CG04AB1234 (Chhattisgarh-style).
        let detection_id = rectified_lineage(ctx)?;
        let text = "CG04AB1234".to_owned();
        let confidences = vec![0.97; text.len()];
        ctx.ocr = vec![OcrHypothesis {
            detection_id,
            text: text.clone(),
            confidence: 0.96,
            char_confidences: confidences,
            provider: Some(ProviderRef {
                name: "fixture".into(),
                model: None,
            }),
        }];
        ctx.push_visual_evidence(Evidence::visual(
            "fixture.s3",
            serde_json::json!({ "text": text }),
            0.96,
        ))?;
        Ok(StageStatus::Continue)
    }
}

fn rectified_lineage(ctx: &FrameContext) -> Result<DetectionId, TalosError> {
    ctx.rectified
        .as_ref()
        .map(|r| r.detection_id.clone())
        .ok_or_else(|| TalosError::Validation("stage requires a RectifiedPlate lineage".into()))
}

/// Minimal Indian plate grammar for fixture spine (state + district + series + number).
pub fn validate_indian_plate(detection_id: &DetectionId, raw: &str) -> GrammarResult {
    let normalized: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    let ok = {
        let bytes = normalized.as_bytes();
        // Pattern: 2 letters + 2 digits + 1–3 letters + 4 digits (common private format).
        let re_ok = normalized.len() >= 8
            && normalized.len() <= 11
            && bytes.first().is_some_and(|b| b.is_ascii_alphabetic())
            && bytes.get(1).is_some_and(|b| b.is_ascii_alphabetic())
            && bytes.get(2).is_some_and(|b| b.is_ascii_digit())
            && bytes.get(3).is_some_and(|b| b.is_ascii_digit())
            && normalized[4..normalized.len().saturating_sub(4)]
                .chars()
                .all(|c| c.is_ascii_alphabetic())
            && normalized.chars().rev().take(4).all(|c| c.is_ascii_digit());
        re_ok
            && !normalized[4..normalized.len().saturating_sub(4)].is_empty()
            && normalized[4..normalized.len().saturating_sub(4)].len() <= 3
    };

    GrammarResult {
        detection_id: detection_id.clone(),
        normalized_text: normalized,
        status: if ok {
            GrammarStatus::Valid
        } else {
            GrammarStatus::Invalid
        },
        errors: if ok {
            vec![]
        } else {
            vec!["does not match Indian private vehicle pattern".into()]
        },
    }
}

#[async_trait]
impl Stage for FixtureGrammar {
    fn id(&self) -> StageId {
        StageId::Grammar
    }
    fn name(&self) -> &'static str {
        "fixture_grammar"
    }
    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
        let detection_id = rectified_lineage(ctx)?;
        let text = ctx.ocr.first().map(|h| h.text.as_str()).unwrap_or("");
        let result = validate_indian_plate(&detection_id, text);
        ctx.grammar = Some(result);
        Ok(StageStatus::Continue)
    }
}

#[async_trait]
impl Stage for FixtureHsrp {
    fn id(&self) -> StageId {
        StageId::Hsrp
    }
    fn name(&self) -> &'static str {
        "fixture_hsrp"
    }
    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
        let detection_id = rectified_lineage(ctx)?;
        ctx.hsrp = Some(HsrpEvidence {
            detection_id,
            score: 0.88,
            ind_mark: CueObservation::Observed,
            hologram: CueObservation::Observed,
            geometry: CueObservation::Observed,
            notes: vec!["fixture".into()],
        });
        ctx.push_visual_evidence(Evidence::visual(
            "fixture.s5",
            serde_json::json!({ "hsrp_score": 0.88 }),
            0.88,
        ))?;
        Ok(StageStatus::Continue)
    }
}

#[async_trait]
impl Stage for FixtureDedup {
    fn id(&self) -> StageId {
        StageId::ObservationDedup
    }
    fn name(&self) -> &'static str {
        "fixture_observation_dedup"
    }
    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
        ctx.observation_dedup = Some(DedupResult {
            is_duplicate: false,
            matched_frame_id: None,
            method: Some("fixture_none".into()),
        });
        Ok(StageStatus::Continue)
    }
}

#[async_trait]
impl Stage for FixtureDecision {
    fn id(&self) -> StageId {
        StageId::Decision
    }
    fn name(&self) -> &'static str {
        "fixture_decision"
    }
    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
        let (plate_text, grammar_ok) = ctx
            .grammar
            .as_ref()
            .map(|g| (g.normalized_text.clone(), g.is_valid()))
            .unwrap_or_default();
        let hsrp_score = ctx.hsrp.as_ref().map(|h| h.score).unwrap_or(0.0);
        let ocr_conf = ctx.ocr.first().map(|o| o.confidence).unwrap_or(0.0);
        let det_conf = ctx
            .detections
            .as_ref()
            .and_then(|d| d.plates.first())
            .map(|p| p.score)
            .unwrap_or(0.0);
        let quality = ctx.quality.as_ref().map(|q| q.score).unwrap_or(0.0);

        let mut hard_fail_reasons = Vec::new();
        if self.decision.hard_fail_on_no_plate
            && ctx
                .detections
                .as_ref()
                .map(|d| d.plates.is_empty())
                .unwrap_or(true)
        {
            hard_fail_reasons.push("no_plate".into());
        }
        if self.decision.hard_fail_on_grammar && !grammar_ok {
            hard_fail_reasons.push("grammar_fail".into());
        }
        let hard_fail = !hard_fail_reasons.is_empty();

        // Simple evidence-driven fusion (not OCR alone).
        let fused_confidence = (0.25 * ocr_conf)
            + (0.25 * det_conf)
            + (0.20 * hsrp_score)
            + (0.15 * quality)
            + (0.15 * if grammar_ok { 1.0 } else { 0.0 });

        let outcome = self.decision.outcome_for(fused_confidence, hard_fail);

        ctx.fused = Some(FusedDecision {
            plate_text,
            grammar_ok,
            hsrp_score,
            fused_confidence,
            outcome,
            hard_fail,
            hard_fail_reasons,
            config_revision_id: None,
        });
        Ok(StageStatus::Continue)
    }
}

/// Stage that always returns NotImplemented (for negative tests / local_gpu_ext).
pub struct NotImplementedStage {
    pub id: StageId,
    pub component: String,
}

#[async_trait]
impl Stage for NotImplementedStage {
    fn id(&self) -> StageId {
        self.id
    }
    fn name(&self) -> &'static str {
        "not_implemented"
    }
    async fn process(&self, _ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
        Err(TalosError::not_implemented(self.component.clone()))
    }
}

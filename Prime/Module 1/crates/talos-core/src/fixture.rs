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
            bbox: BoundingBox {
                x: 100.0,
                y: 200.0,
                w: 240.0,
                h: 60.0,
            },
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
                bbox: BoundingBox {
                    x: 40.0,
                    y: 80.0,
                    w: 500.0,
                    h: 360.0,
                },
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
        let bbox = ctx
            .detections
            .as_ref()
            .and_then(|d| d.plates.first())
            .map(|p| p.bbox.clone())
            .unwrap_or(BoundingBox {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            });
        ctx.rectified = Some(RectifiedPlate {
            bbox,
            crop_ref: format!("fixture://crop/{}", ctx.intake.frame_id),
            width: 240,
            height: 60,
            detection_id: ctx
                .detections
                .as_ref()
                .and_then(|d| d.plates.first())
                .and_then(|p| p.detection_id.clone()),
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
        let text = "CG04AB1234".to_owned();
        let confidences = vec![0.97; text.len()];
        ctx.ocr = vec![OcrHypothesis {
            text: text.clone(),
            confidence: 0.96,
            char_confidences: confidences,
            detection_id: ctx.rectified.as_ref().and_then(|r| r.detection_id.clone()),
        }];
        ctx.push_visual_evidence(Evidence::visual(
            "fixture.s3",
            serde_json::json!({ "text": text }),
            0.96,
        ))?;
        Ok(StageStatus::Continue)
    }
}

/// Minimal Indian plate grammar for fixture spine (state + district + series + number).
pub fn validate_indian_plate(raw: &str) -> GrammarResult {
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
        normalized_text: normalized,
        ok,
        errors: if ok {
            vec![]
        } else {
            vec!["does not match Indian private vehicle pattern".into()]
        },
        detection_id: None,
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
        let text = ctx.ocr.first().map(|h| h.text.as_str()).unwrap_or("");
        let result = validate_indian_plate(text);
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
        ctx.hsrp = Some(HsrpEvidence {
            score: 0.88,
            ind_mark_detected: true,
            hologram_cues: true,
            geometry_ok: true,
            notes: vec!["fixture".into()],
            detection_id: ctx.rectified.as_ref().and_then(|r| r.detection_id.clone()),
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
        let grammar = ctx.grammar.clone().unwrap_or(GrammarResult {
            normalized_text: String::new(),
            ok: false,
            errors: vec!["missing grammar".into()],
            detection_id: None,
        });
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
        if self.decision.hard_fail_on_grammar && !grammar.ok {
            hard_fail_reasons.push("grammar_fail".into());
        }
        let hard_fail = !hard_fail_reasons.is_empty();

        // Simple evidence-driven fusion (not OCR alone).
        let fused_confidence = (0.25 * ocr_conf)
            + (0.25 * det_conf)
            + (0.20 * hsrp_score)
            + (0.15 * quality)
            + (0.15 * if grammar.ok { 1.0 } else { 0.0 });

        let outcome = self.decision.outcome_for(fused_confidence, hard_fail);

        ctx.fused = Some(FusedDecision {
            plate_text: grammar.normalized_text.clone(),
            grammar_ok: grammar.ok,
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

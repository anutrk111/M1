//! Assemble Stages 0–7 from `PipelineConfig` backends.
//!
//! `ai_api` stages 1/3/5 are thin adapters over the M01 [`VisionGateway`] port:
//! they forward requests and store the normalized result. They contain no
//! M05–M08 logic (no DetectionId minting, suppression, grammar, or fusion);
//! those modules replace the fixture stages as they land.

use crate::error::TalosError;
use crate::fixture::*;
use crate::pipeline::{FrameContext, Pipeline, Stage, StageId, StageStatus};
use crate::ports::{VisionGateway, VisionRequest};
use async_trait::async_trait;
use std::sync::Arc;
use talos_config::{DecisionConfig, PipelineConfig, StageBackend};
use talos_types::Evidence;

pub fn build_pipeline(
    pipeline: &PipelineConfig,
    decision: DecisionConfig,
    gateway: Option<Arc<dyn VisionGateway>>,
) -> Result<Pipeline, TalosError> {
    let mut stages: Vec<Arc<dyn Stage>> = Vec::with_capacity(StageId::ALL.len());
    for id in StageId::ALL {
        let stage: Arc<dyn Stage> = match pipeline.backend_for(id.index()) {
            StageBackend::Fixture => fixture_stage(id, &decision),
            StageBackend::LocalGpuExt => Arc::new(NotImplementedStage {
                id,
                component: format!("local_gpu_ext for stage {}", id.name()),
            }),
            StageBackend::AiApi => {
                let needs_gateway = matches!(id, StageId::Detection | StageId::Ocr | StageId::Hsrp);
                match (needs_gateway, gateway.clone()) {
                    (true, Some(gw)) => Arc::new(GatewayStage { id, gateway: gw }),
                    (true, None) => {
                        return Err(TalosError::Config(format!(
                            "stage {} uses ai_api but no VisionGateway is configured",
                            id.name()
                        )))
                    }
                    (false, _) => Arc::new(NotImplementedStage {
                        id,
                        component: format!(
                            "ai_api backend for stage {} (owned by its module)",
                            id.name()
                        ),
                    }),
                }
            }
        };
        stages.push(stage);
    }
    Pipeline::new(stages)
}

fn fixture_stage(id: StageId, decision: &DecisionConfig) -> Arc<dyn Stage> {
    match id {
        StageId::ImageQuality => Arc::new(FixtureImageQuality),
        StageId::Detection => Arc::new(FixtureDetection),
        StageId::Rectification => Arc::new(FixtureRectification),
        StageId::Ocr => Arc::new(FixtureOcr),
        StageId::Grammar => Arc::new(FixtureGrammar),
        StageId::Hsrp => Arc::new(FixtureHsrp),
        StageId::ObservationDedup => Arc::new(FixtureDedup),
        StageId::Decision => Arc::new(FixtureDecision {
            decision: decision.clone(),
        }),
    }
}

/// Pass-through adapter for stages 1 (detect), 3 (OCR) and 5 (HSRP).
pub struct GatewayStage {
    pub id: StageId,
    pub gateway: Arc<dyn VisionGateway>,
}

impl GatewayStage {
    fn plate_request(&self, ctx: &FrameContext) -> Result<VisionRequest, TalosError> {
        let plate = ctx.rectified.as_ref().ok_or_else(|| {
            TalosError::Validation(format!(
                "stage {} requires a RectifiedPlate lineage",
                self.id.name()
            ))
        })?;
        Ok(VisionRequest::for_plate(
            &ctx.intake,
            ctx.trace_id.clone(),
            plate,
        ))
    }
}

#[async_trait]
impl Stage for GatewayStage {
    fn id(&self) -> StageId {
        self.id
    }

    fn name(&self) -> &'static str {
        match self.id {
            StageId::Detection => "gateway_detection",
            StageId::Ocr => "gateway_ocr",
            StageId::Hsrp => "gateway_hsrp",
            _ => "gateway_unsupported",
        }
    }

    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
        match self.id {
            StageId::Detection => {
                let req = VisionRequest::for_frame(&ctx.intake, ctx.trace_id.clone());
                let det = self.gateway.detect_vehicles_plates(req).await?;
                let plates = det.plates.len();
                ctx.detections = Some(det);
                ctx.push_visual_evidence(Evidence::visual(
                    "gateway.s1",
                    serde_json::json!({ "plates": plates }),
                    1.0,
                ))?;
            }
            StageId::Ocr => {
                let req = self.plate_request(ctx)?;
                ctx.ocr = self.gateway.run_ocr(req).await?;
            }
            StageId::Hsrp => {
                let req = self.plate_request(ctx)?;
                let hsrp = self.gateway.analyze_hsrp(req).await?;
                let score = hsrp.score;
                ctx.hsrp = Some(hsrp);
                ctx.push_visual_evidence(Evidence::visual(
                    "gateway.s5",
                    serde_json::json!({ "hsrp_score": score }),
                    score,
                ))?;
            }
            other => {
                return Err(TalosError::Internal(format!(
                    "GatewayStage does not serve stage {}",
                    other.name()
                )))
            }
        }
        Ok(StageStatus::Continue)
    }
}

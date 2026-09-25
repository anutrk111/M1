//! Stage backend wiring against the M01 VisionGateway port (stub adapter).

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use talos_config::{DecisionConfig, PipelineConfig, StageBackend};
use talos_core::backend::build_pipeline;
use talos_core::pipeline::FrameContext;
use talos_core::util::{new_batch_id, new_frame_id, new_trace_id};
use talos_core::{TalosError, VisionGateway, VisionRequest, VlmAssistResult, VlmRequest};
use talos_types::*;

#[derive(Default)]
struct StubGateway {
    ocr_calls: AtomicUsize,
    hsrp_calls: AtomicUsize,
}

#[async_trait]
impl VisionGateway for StubGateway {
    async fn detect_vehicles_plates(&self, req: VisionRequest) -> Result<Detections, TalosError> {
        req.validate()?;
        Ok(Detections {
            vehicles: vec![],
            plates: vec![Detection {
                label: "plate".into(),
                score: 0.9,
                bbox: BoundingBox::new(0.1, 0.4, 0.3, 0.46).unwrap(),
                detection_id: None,
                provider: Some(ProviderRef {
                    name: "stub".into(),
                    model: None,
                }),
            }],
            summary: None,
        })
    }

    async fn run_ocr(&self, req: VisionRequest) -> Result<Vec<OcrHypothesis>, TalosError> {
        self.ocr_calls.fetch_add(1, Ordering::SeqCst);
        let det = req.require_detection_id()?.clone();
        Ok(vec![OcrHypothesis {
            detection_id: det,
            text: "CG04AB1234".into(),
            confidence: 0.95,
            char_confidences: vec![],
            provider: Some(ProviderRef {
                name: "stub".into(),
                model: None,
            }),
        }])
    }

    async fn analyze_hsrp(&self, req: VisionRequest) -> Result<HsrpEvidence, TalosError> {
        self.hsrp_calls.fetch_add(1, Ordering::SeqCst);
        Ok(HsrpEvidence {
            detection_id: req.require_detection_id()?.clone(),
            score: 0.7,
            ind_mark: CueObservation::Observed,
            hologram: CueObservation::NotObservable,
            geometry: CueObservation::Observed,
            notes: vec![],
        })
    }

    async fn vlm_assist(&self, _req: VlmRequest) -> Result<VlmAssistResult, TalosError> {
        Err(TalosError::not_implemented("stub vlm"))
    }
}

fn ctx() -> FrameContext {
    FrameContext::new(
        IntakeEnvelope::new(
            new_batch_id(),
            new_frame_id(),
            SourceRef {
                kind: SourceKind::Folder,
                path_or_key: "a.jpg".into(),
            },
            ImageRef {
                content_type: "image/jpeg".into(),
                sha256: "00".into(),
                bytes_ref: "file:///staging/a.jpg".into(),
            },
        ),
        new_trace_id(),
    )
}

fn cfg(overrides: &[(u8, StageBackend)]) -> PipelineConfig {
    let mut c = PipelineConfig::defaults();
    for (s, b) in overrides {
        let slot = match s {
            0 => &mut c.stages.s0,
            1 => &mut c.stages.s1,
            2 => &mut c.stages.s2,
            3 => &mut c.stages.s3,
            4 => &mut c.stages.s4,
            5 => &mut c.stages.s5,
            6 => &mut c.stages.s6,
            _ => &mut c.stages.s7,
        };
        *slot = Some(*b);
    }
    c
}

#[tokio::test]
async fn all_fixture_backends_match_fixture_pipeline() {
    let p = build_pipeline(
        &PipelineConfig::defaults(),
        DecisionConfig::defaults(),
        None,
    )
    .unwrap();
    assert_eq!(p.stages().len(), 8);
    let mut c = ctx();
    p.run(&mut c).await.unwrap();
    assert_eq!(c.terminal_status, Some(FrameTerminalStatus::Succeeded));
}

#[tokio::test]
async fn ai_api_ocr_and_hsrp_route_through_gateway_with_lineage() {
    let gw = Arc::new(StubGateway::default());
    let p = build_pipeline(
        &cfg(&[(3, StageBackend::AiApi), (5, StageBackend::AiApi)]),
        DecisionConfig::defaults(),
        Some(gw.clone()),
    )
    .unwrap();
    let mut c = ctx();
    p.run(&mut c).await.unwrap();
    assert_eq!(gw.ocr_calls.load(Ordering::SeqCst), 1);
    assert_eq!(gw.hsrp_calls.load(Ordering::SeqCst), 1);
    let lineage = c.rectified.as_ref().unwrap().detection_id.clone();
    assert_eq!(c.ocr[0].detection_id, lineage);
    assert_eq!(c.hsrp.as_ref().unwrap().detection_id, lineage);
    assert_eq!(
        c.hsrp.as_ref().unwrap().hologram,
        CueObservation::NotObservable
    );
    assert_eq!(c.terminal_status, Some(FrameTerminalStatus::Succeeded));
}

#[tokio::test]
async fn ai_api_detection_without_m05_minting_fails_closed_downstream() {
    let gw: Arc<dyn VisionGateway> = Arc::new(StubGateway::default());
    let p = build_pipeline(
        &cfg(&[(1, StageBackend::AiApi)]),
        DecisionConfig::defaults(),
        Some(gw),
    )
    .unwrap();
    let mut c = ctx();
    let err = p.run(&mut c).await.unwrap_err();
    assert!(matches!(err, TalosError::Validation(_)), "{err}");
    assert_eq!(
        c.terminal_status,
        Some(FrameTerminalStatus::FailedValidation)
    );
    assert!(c.fused.is_none(), "no fake decision");
}

#[test]
fn ai_api_without_gateway_is_config_error() {
    let err = build_pipeline(
        &cfg(&[(1, StageBackend::AiApi)]),
        DecisionConfig::defaults(),
        None,
    )
    .err()
    .unwrap();
    assert!(matches!(err, TalosError::Config(_)));
}

#[tokio::test]
async fn ai_api_on_non_gateway_stage_and_local_gpu_are_not_implemented() {
    for (stage, backend) in [(4, StageBackend::AiApi), (0, StageBackend::LocalGpuExt)] {
        let p = build_pipeline(
            &cfg(&[(stage, backend)]),
            DecisionConfig::defaults(),
            Some(Arc::new(StubGateway::default())),
        )
        .unwrap();
        let mut c = ctx();
        let err = p.run(&mut c).await.unwrap_err();
        assert!(matches!(err, TalosError::NotImplemented(_)));
        assert_eq!(
            c.terminal_status,
            Some(FrameTerminalStatus::FailedNotImplemented)
        );
    }
}

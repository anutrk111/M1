//! Integration: fixture Stages 0–7 → schema-valid decision JSON (no GPU/network).

use talos_core::fixture::{fixture_pipeline, validate_indian_plate, NotImplementedStage};
use talos_core::pipeline::{FrameContext, Pipeline, Stage, StageId};
use talos_core::util::{new_batch_id, new_frame_id, new_trace_id, sha256_hex};
use talos_core::TalosError;
use talos_config::DecisionConfig;
use talos_types::*;
use std::sync::Arc;

fn sample_intake() -> IntakeEnvelope {
    IntakeEnvelope::new(
        new_batch_id(),
        new_frame_id(),
        SourceRef {
            kind: SourceKind::Folder,
            path_or_key: "tests/fixtures/sample.jpg".into(),
        },
        ImageRef {
            content_type: "image/jpeg".into(),
            sha256: sha256_hex(b"fixture"),
            bytes_ref: "fixture://sample.jpg".into(),
        },
    )
}

#[tokio::test]
async fn fixture_pipeline_emits_decision_json() {
    let decision_cfg = DecisionConfig::defaults();
    let pipeline = fixture_pipeline(decision_cfg).unwrap();
    let mut ctx = FrameContext::new(sample_intake(), new_trace_id());
    pipeline.run(&mut ctx).await.unwrap();

    assert_eq!(
        ctx.terminal_status,
        Some(FrameTerminalStatus::Succeeded)
    );

    let json = ctx.to_decision_json().unwrap();
    assert_eq!(json.schema_version, SCHEMA_VERSION);
    assert!(json.grammar_ok);
    assert_eq!(json.plate_text, "CG04AB1234");
    assert!(json.fused_confidence > 0.0);
    assert!(!json.evidence.is_empty());
    assert!(json
        .evidence
        .iter()
        .all(|e| e.kind == EvidenceKind::VisualObservation));
    assert_eq!(json.stages_ms.len(), 8);
    assert!(matches!(
        json.outcome,
        DecisionOutcome::AutoApproved
            | DecisionOutcome::SecondaryVerification
            | DecisionOutcome::ReviewRequired
    ));

    let encoded = serde_json::to_value(&json).unwrap();
    assert_eq!(encoded["schema_version"], "2.0");
    assert!(encoded.get("outcome").is_some());
}

#[tokio::test]
async fn not_implemented_sets_terminal_status() {
    let stages: Vec<Arc<dyn Stage>> = vec![Arc::new(NotImplementedStage {
        id: StageId::ImageQuality,
        component: "local_gpu_ext".into(),
    })];
    let pipeline = Pipeline::new(stages).unwrap();
    let mut ctx = FrameContext::new(sample_intake(), new_trace_id());
    let err = pipeline.run(&mut ctx).await.unwrap_err();
    assert!(matches!(err, TalosError::NotImplemented(_)));
    assert_eq!(
        ctx.terminal_status,
        Some(FrameTerminalStatus::FailedNotImplemented)
    );
}

#[test]
fn indian_plate_grammar_fixture() {
    assert!(validate_indian_plate("CG04AB1234").ok);
    assert!(validate_indian_plate("cg-04-ab-1234").ok);
    assert!(!validate_indian_plate("INVALID").ok);
    assert!(!validate_indian_plate("").ok);
}

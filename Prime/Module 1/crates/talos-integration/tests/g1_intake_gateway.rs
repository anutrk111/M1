//! Gate G1: M02 intake → M01 envelope / `VisionGateway` port → M03 `AiGateway`
//! (fixture provider). Offline: no network, no paid APIs, no M04–M12 logic.
//!
//! `DetectionId` minting is M05. Where a plate lineage is needed the test mints a
//! stand-in id and uses the staged original as the crop, and says so.

use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use talos_ai_gateway::{AiConfig, AiGateway, InMemoryCostRecorder};
use talos_config::{DecisionConfig, PipelineConfig, StageBackend};
use talos_core::backend::build_pipeline;
use talos_core::fixture::validate_indian_plate;
use talos_core::util::{new_trace_id, sha256_hex};
use talos_core::{FrameContext, TalosError, VisionGateway, VisionRequest};
use talos_intake::{
    import_folder, import_upload, BatchOutcome, InMemoryFrameSink, IntakeConfig, IntakeStatus,
    UploadedFile,
};
use talos_types::*;

fn jpeg(r: u8, g: u8, b: u8) -> Vec<u8> {
    use image::{ImageBuffer, ImageFormat, Rgb};
    let img = ImageBuffer::from_pixel(32, 16, Rgb([r, g, b]));
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
        .unwrap();
    buf
}

fn intake_cfg(dir: &Path) -> IntakeConfig {
    IntakeConfig {
        staging_root: dir.join("staging"),
        max_images_per_batch: 100,
        fail_on_empty_batch: true,
        ..Default::default()
    }
}

/// Fixture gateway authorized to read only the M02 staging root under `dir`.
fn gateway(dir: &Path) -> (Arc<AiGateway>, Arc<InMemoryCostRecorder>) {
    let cost = Arc::new(InMemoryCostRecorder::default());
    let mut cfg = AiConfig::fixture_defaults();
    cfg.local_artifact_roots = vec![intake_cfg(dir).staging_root];
    let gw = AiGateway::from_config(cfg, cost.clone()).unwrap();
    (Arc::new(gw), cost)
}

fn staged_path(env: &IntakeEnvelope) -> &Path {
    Path::new(
        env.image
            .bytes_ref
            .strip_prefix("file://")
            .expect("M02 emits file:// staging refs"),
    )
}

async fn upload_one(dir: &Path, bytes: Vec<u8>) -> IntakeEnvelope {
    let sink = Arc::new(InMemoryFrameSink::new());
    let res = import_upload(
        vec![UploadedFile::new("plate.jpg", bytes)],
        &intake_cfg(dir),
        sink.clone(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(res.outcome, BatchOutcome::Complete);
    sink.envelopes().await.remove(0)
}

#[tokio::test]
async fn folder_intake_to_gateway_detect_preserves_contracts_and_lineage() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(root.join("cam01")).unwrap();
    let a = jpeg(10, 20, 30);
    let b = jpeg(40, 50, 60);
    fs::write(root.join("cam01/a.jpg"), &a).unwrap();
    fs::write(root.join("cam01/b.jpg"), &b).unwrap();
    fs::write(root.join("cam01/b_dup.jpg"), &b).unwrap();
    fs::write(root.join("notes.txt"), b"ignored").unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let res = import_folder(&root, &intake_cfg(tmp.path()), sink.clone(), None)
        .await
        .unwrap();
    let m = &res.manifest;
    assert!(m.reconcile_ok());
    assert_eq!(m.counts.accepted, 2);
    assert_eq!(m.counts.skipped_duplicate, 1);
    assert_eq!(m.counts.ignored_unsupported, 1);

    let envelopes = sink.envelopes().await;
    assert_eq!(envelopes.len(), 2);
    let originals: HashSet<String> = [sha256_hex(&a), sha256_hex(&b)].into();

    let (gw, cost) = gateway(tmp.path());
    let port: Arc<dyn VisionGateway> = gw.clone();
    for env in &envelopes {
        assert_eq!(env.schema_version, INTAKE_SCHEMA_VERSION);
        assert_eq!(env.batch_id, m.batch_id);
        assert_eq!(env.source.kind, SourceKind::Folder);
        let round: IntakeEnvelope =
            serde_json::from_str(&serde_json::to_string(env).unwrap()).unwrap();
        assert_eq!(&round, env);

        // SHA is over the original bytes; the staged copy is byte-identical.
        assert!(originals.contains(&env.image.sha256));
        assert_eq!(
            sha256_hex(&fs::read(staged_path(env)).unwrap()),
            env.image.sha256
        );

        let trace = new_trace_id();
        let req = VisionRequest::for_frame(env, trace.clone());
        assert_eq!(req.frame_id, env.frame_id);
        assert_eq!(req.batch_id, env.batch_id);
        assert!(req.detection_id.is_none());

        let det = port.detect_vehicles_plates(req).await.unwrap();
        assert!(!det.plates.is_empty());
        for p in &det.plates {
            assert!(p.detection_id.is_none(), "M05 mints DetectionIds, not M03");
            p.bbox.validate().unwrap();
            assert!(p.provider.is_some());
        }
        let encoded = serde_json::to_string(&det).unwrap();
        assert_eq!(serde_json::from_str::<Detections>(&encoded).unwrap(), det);

        let ev = cost
            .events()
            .into_iter()
            .find(|e| e.trace_id == trace)
            .expect("one cost event per gateway call");
        assert_eq!(ev.frame_id, env.frame_id);
        assert_eq!(ev.batch_id, env.batch_id);
        assert_eq!(ev.operation, AiOperation::DetectVehiclesPlates);
        assert_eq!(ev.status, CostStatus::Ok);
    }
    assert_eq!(cost.events().len(), 2);

    // Originals are never mutated by intake or gateway reads.
    assert_eq!(fs::read(root.join("cam01/a.jpg")).unwrap(), a);
    assert_eq!(fs::read(root.join("cam01/b.jpg")).unwrap(), b);
    let requests: u64 = gw
        .metrics()
        .counters
        .iter()
        .filter(|(k, _)| k.contains("operation=detect_vehicles_plates") && k.contains("status=ok"))
        .map(|(_, v)| *v)
        .sum();
    assert_eq!(requests, 2);
}

#[tokio::test]
async fn upload_intake_plate_ocr_and_hsrp_are_bound_to_detection_id() {
    let tmp = tempfile::tempdir().unwrap();
    let env = upload_one(tmp.path(), jpeg(1, 2, 3)).await;
    assert_eq!(env.source.kind, SourceKind::Upload);

    let (gw, cost) = gateway(tmp.path());
    let trace = new_trace_id();
    let det = gw
        .detect_vehicles_plates(VisionRequest::for_frame(&env, trace.clone()))
        .await
        .unwrap();
    let plate = &det.plates[0];

    // Stand-in for M05 minting + M04 rectification (not implemented in this gate).
    let detection_id = DetectionId::new(format!("{}-p0", env.frame_id));
    let rectified = RectifiedPlate {
        detection_id: detection_id.clone(),
        bbox: plate.bbox,
        crop_ref: env.image.bytes_ref.clone(),
        width: 240,
        height: 60,
    };
    let req = VisionRequest::for_plate(&env, trace.clone(), &rectified);
    assert_eq!(req.detection_id.as_ref(), Some(&detection_id));

    let consensus = gw.run_ocr_consensus(req.clone()).await.unwrap();
    let candidates = consensus.all();
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|h| h.detection_id == detection_id));

    let top = candidates[0].clone();
    let ocr = OcrResult {
        detection_id: detection_id.clone(),
        grammar: validate_indian_plate(&detection_id, &top.text),
        selected: Some(top),
        candidates,
    };
    ocr.validate_lineage().unwrap();

    let hsrp = gw.analyze_hsrp(req).await.unwrap();
    assert_eq!(hsrp.detection_id, detection_id);
    let json = serde_json::to_value(&hsrp).unwrap();
    for cue in ["ind_mark", "hologram", "geometry"] {
        let v = json[cue].as_str().unwrap();
        assert!(
            ["OBSERVED", "NOT_OBSERVED", "NOT_OBSERVABLE"].contains(&v),
            "{cue} must be ternary, got {v}"
        );
    }

    let events = cost.events();
    assert!(events
        .iter()
        .all(|e| e.frame_id == env.frame_id && e.batch_id == env.batch_id && e.trace_id == trace));
    let ops: HashSet<AiOperation> = events.iter().map(|e| e.operation).collect();
    assert!(ops.contains(&AiOperation::RunOcr));
    assert!(ops.contains(&AiOperation::AnalyzeHsrp));
}

#[tokio::test]
async fn ocr_and_hsrp_without_detection_id_are_rejected_before_egress() {
    let tmp = tempfile::tempdir().unwrap();
    let env = upload_one(tmp.path(), jpeg(9, 9, 9)).await;
    let (gw, cost) = gateway(tmp.path());

    let frame_req = VisionRequest::for_frame(&env, new_trace_id());
    assert!(matches!(
        gw.run_ocr(frame_req.clone()).await,
        Err(TalosError::Validation(_))
    ));
    assert!(matches!(
        gw.analyze_hsrp(frame_req).await,
        Err(TalosError::Validation(_))
    ));
    assert!(cost.events().is_empty(), "no provider call, no cost");
}

#[tokio::test]
async fn fixture_pipeline_accepts_real_m02_envelope() {
    let tmp = tempfile::tempdir().unwrap();
    let env = upload_one(tmp.path(), jpeg(5, 5, 5)).await;
    let p = build_pipeline(
        &PipelineConfig::defaults(),
        DecisionConfig::defaults(),
        None,
    )
    .unwrap();
    let mut ctx = FrameContext::new(env.clone(), new_trace_id());
    p.run(&mut ctx).await.unwrap();
    assert_eq!(ctx.terminal_status, Some(FrameTerminalStatus::Succeeded));
    let decision = ctx.to_decision_json().unwrap();
    assert_eq!(decision.schema_version, DECISION_SCHEMA_VERSION);
    assert_eq!(decision.batch_id, env.batch_id);
    assert_eq!(decision.frame_id, env.frame_id);
}

#[tokio::test]
async fn ai_api_pipeline_with_real_gateway_fails_closed_until_m04_m05_land() {
    let tmp = tempfile::tempdir().unwrap();
    let env = upload_one(tmp.path(), jpeg(7, 7, 7)).await;
    let (gw, _cost) = gateway(tmp.path());

    let mut cases = Vec::new();
    for (stage, expected) in [
        // Gateway detections carry no DetectionId; fixture rectification refuses them.
        (1u8, FrameTerminalStatus::FailedValidation),
        // Fixture rectification emits `fixture://` crops the gateway cannot resolve.
        (3u8, FrameTerminalStatus::FailedNotImplemented),
    ] {
        let mut cfg = PipelineConfig::defaults();
        match stage {
            1 => cfg.stages.s1 = Some(StageBackend::AiApi),
            _ => cfg.stages.s3 = Some(StageBackend::AiApi),
        }
        let p = build_pipeline(&cfg, DecisionConfig::defaults(), Some(gw.clone())).unwrap();
        let mut ctx = FrameContext::new(env.clone(), new_trace_id());
        let err = p.run(&mut ctx).await.unwrap_err();
        cases.push((stage, err.to_string()));
        assert_eq!(ctx.terminal_status, Some(expected), "stage {stage}: {err}");
        assert!(ctx.fused.is_none(), "stage {stage}: no fake decision");
        assert!(ctx.to_decision_json().is_err());
    }
    assert_eq!(cases.len(), 2);
}

#[tokio::test]
async fn duplicate_upload_is_recorded_not_sent_to_gateway() {
    let tmp = tempfile::tempdir().unwrap();
    let bytes = jpeg(3, 3, 3);
    let sink = Arc::new(InMemoryFrameSink::new());
    let res = import_upload(
        vec![
            UploadedFile::new("a.jpg", bytes.clone()),
            UploadedFile::new("b.jpg", bytes),
        ],
        &intake_cfg(tmp.path()),
        sink.clone(),
        None,
    )
    .await
    .unwrap();
    assert!(res.manifest.reconcile_ok());
    let dup = res
        .manifest
        .frames
        .iter()
        .find(|f| f.intake_status == IntakeStatus::SkippedDuplicate)
        .unwrap();
    let envelopes = sink.envelopes().await;
    assert_eq!(envelopes.len(), 1);
    assert_eq!(
        dup.duplicate_of.as_ref().unwrap().original_frame_id,
        envelopes[0].frame_id
    );
}

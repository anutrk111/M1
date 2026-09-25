//! Wave 1 hardening: durable sink boundary, upload adapter, recovery, staging plan.

use async_trait::async_trait;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use talos_core::util::sha256_hex;
use talos_core::TalosError;
use talos_intake::{
    import_folder, import_source, import_upload, import_zip, plan_cleanup, BatchManifest,
    BatchOutcome, CleanupReason, DurableSink, DurableSinkAdapter, FailingFrameSink, IdempotencyKey,
    InMemoryDurableSink, InMemoryFrameSink, IntakeConfig, IntakeSource, IntakeStatus,
    RecoveryContext, SinkAck, StagingPolicy, UploadedFile,
};
use talos_types::{IntakeEnvelope, SourceKind};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn jpeg(r: u8, g: u8, b: u8) -> Vec<u8> {
    use image::{ImageBuffer, ImageFormat, Rgb};
    let img = ImageBuffer::from_pixel(16, 16, Rgb([r, g, b]));
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
        .unwrap();
    buf
}

fn cfg(dir: &Path) -> IntakeConfig {
    IntakeConfig {
        staging_root: dir.join("staging"),
        max_images_per_batch: 100,
        fail_on_empty_batch: true,
        ..Default::default()
    }
}

fn write_zip(path: &PathBuf, entries: &[(&str, &[u8])]) {
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in entries {
        zip.start_file(*name, opts).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
}

fn assert_counts_reconcile(m: &BatchManifest) {
    let c = &m.counts;
    assert_eq!(
        c.image_candidates_discovered,
        c.accepted + c.rejected + c.skipped_duplicate + c.failed_sink + c.skipped_already_accepted
    );
    assert_eq!(
        c.filesystem_entries_seen,
        c.image_candidates_discovered + c.ignored_unsupported
    );
    assert!(m.reconcile_ok());
}

// ---------------------------------------------------------------- corrupt ZIP

#[tokio::test]
async fn corrupt_zip_central_directory_rejected_with_validation() {
    let tmp = tempfile::tempdir().unwrap();
    let good = tmp.path().join("good.zip");
    write_zip(
        &good,
        &[("a.jpg", &jpeg(1, 2, 3)), ("b.jpg", &jpeg(4, 5, 6))],
    );
    let bytes = fs::read(&good).unwrap();

    // 1) Central-directory file headers (PK\x01\x02) smashed.
    let mut smashed = bytes.clone();
    let mut hits = 0;
    for i in 0..smashed.len().saturating_sub(4) {
        if smashed[i..i + 4] == [b'P', b'K', 0x01, 0x02] {
            smashed[i..i + 4].copy_from_slice(b"XXXX");
            hits += 1;
        }
    }
    assert_eq!(hits, 2, "fixture should contain two CD headers");
    // 2) End-of-central-directory record truncated away.
    let truncated = bytes[..bytes.len() - 22].to_vec();
    // 3) Central-directory offset in EOCD pointed past EOF.
    let mut bad_offset = bytes.clone();
    let eocd = bad_offset.len() - 22;
    bad_offset[eocd + 16..eocd + 20].copy_from_slice(&u32::MAX.to_le_bytes());

    for (label, payload) in [
        ("smashed", smashed),
        ("truncated", truncated),
        ("bad_offset", bad_offset),
    ] {
        let path = tmp.path().join(format!("{label}.zip"));
        fs::write(&path, payload).unwrap();
        let sink = Arc::new(InMemoryFrameSink::new());
        let err = import_zip(&path, &cfg(tmp.path()), sink.clone(), None)
            .await
            .unwrap_err();
        assert!(
            matches!(err, TalosError::Validation(_)),
            "{label}: expected Validation, got {err:?}"
        );
        assert!(sink.is_empty().await, "{label}: nothing may reach the sink");
    }
}

// ---------------------------------------------------------------- recovery

#[tokio::test]
async fn recovery_rerun_emits_nothing_and_counts_skipped_already_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(root.join("cam")).unwrap();
    fs::write(root.join("cam/a.jpg"), jpeg(1, 0, 0)).unwrap();
    fs::write(root.join("cam/b.jpg"), jpeg(2, 0, 0)).unwrap();
    fs::write(root.join("cam/b_copy.jpg"), jpeg(2, 0, 0)).unwrap();
    fs::write(root.join("notes.txt"), b"x").unwrap();
    let config = cfg(tmp.path());

    let first_sink = Arc::new(InMemoryFrameSink::new());
    let first = import_folder(&root, &config, first_sink.clone(), None)
        .await
        .unwrap();
    assert_eq!(first.manifest.counts.accepted, 2);
    assert_eq!(first.manifest.counts.skipped_duplicate, 1);
    assert_eq!(first_sink.len().await, 2);

    let ctx = RecoveryContext::from_manifest(&first.manifest);
    assert_eq!(ctx.accepted_len(), 2);
    let second_sink = Arc::new(InMemoryFrameSink::new());
    let second = import_source(
        IntakeSource::Folder(root.clone()),
        &config,
        second_sink.clone(),
        None,
        Some(&ctx),
    )
    .await
    .unwrap();

    assert_eq!(second_sink.len().await, 0, "no frame may be re-emitted");
    let c = &second.manifest.counts;
    assert_eq!(c.accepted, 0);
    assert_eq!(c.skipped_already_accepted, 2);
    assert_eq!(c.skipped_duplicate, 1);
    assert_eq!(c.image_candidates_discovered, 3);
    assert_eq!(c.ignored_unsupported, 1);
    assert_counts_reconcile(&second.manifest);
    assert_eq!(second.outcome, BatchOutcome::Complete);
    assert_eq!(second.manifest.batch_id, first.manifest.batch_id);

    // Skipped frames carry the prior FrameIds; duplicate references the prior original.
    let prior_ids: HashSet<_> = first
        .manifest
        .frames
        .iter()
        .filter(|f| f.intake_status == IntakeStatus::Accepted)
        .map(|f| f.frame_id.clone().unwrap())
        .collect();
    let skipped_ids: HashSet<_> = second
        .manifest
        .frames
        .iter()
        .filter(|f| f.intake_status == IntakeStatus::SkippedAlreadyAccepted)
        .map(|f| f.frame_id.clone().unwrap())
        .collect();
    assert_eq!(prior_ids, skipped_ids);
    let dup = second
        .manifest
        .frames
        .iter()
        .find(|f| f.intake_status == IntakeStatus::SkippedDuplicate)
        .unwrap();
    assert!(prior_ids.contains(&dup.duplicate_of.as_ref().unwrap().original_frame_id));

    // Chained recovery from the second manifest is still a no-op.
    let third_sink = Arc::new(InMemoryFrameSink::new());
    let third = import_source(
        IntakeSource::Folder(root),
        &config,
        third_sink.clone(),
        None,
        Some(&RecoveryContext::from_manifest(&second.manifest)),
    )
    .await
    .unwrap();
    assert_eq!(third_sink.len().await, 0);
    assert_eq!(third.manifest.counts.skipped_already_accepted, 2);
    assert_counts_reconcile(&third.manifest);
}

#[tokio::test]
async fn recovery_retries_failed_sink_with_same_frame_id() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.jpg"), jpeg(9, 9, 9)).unwrap();
    let mut config = cfg(tmp.path());
    config.fail_on_empty_batch = false;

    let first = import_folder(&root, &config, Arc::new(FailingFrameSink), None)
        .await
        .unwrap();
    assert_eq!(first.manifest.counts.failed_sink, 1);
    let failed_id = first.manifest.frames[0].frame_id.clone().unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let second = import_source(
        IntakeSource::Folder(root),
        &config,
        sink.clone(),
        None,
        Some(&RecoveryContext::from_manifest(&first.manifest)),
    )
    .await
    .unwrap();
    assert_eq!(second.manifest.counts.accepted, 1);
    assert_eq!(second.manifest.counts.skipped_already_accepted, 0);
    assert_eq!(sink.envelopes().await[0].frame_id, failed_id);
    assert_counts_reconcile(&second.manifest);
    assert_eq!(second.outcome, BatchOutcome::Complete);
}

#[tokio::test]
async fn recovery_zip_rerun_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("b.zip");
    write_zip(
        &zip_path,
        &[("cam/x.jpg", &jpeg(3, 3, 3)), ("cam/y.jpg", &jpeg(4, 4, 4))],
    );
    let config = cfg(tmp.path());
    let first = import_zip(&zip_path, &config, Arc::new(InMemoryFrameSink::new()), None)
        .await
        .unwrap();
    assert_eq!(first.manifest.counts.accepted, 2);

    let sink = Arc::new(InMemoryFrameSink::new());
    let second = import_source(
        IntakeSource::Zip(zip_path),
        &config,
        sink.clone(),
        None,
        Some(&RecoveryContext::from_manifest(&first.manifest)),
    )
    .await
    .unwrap();
    assert!(sink.is_empty().await);
    assert_eq!(second.manifest.counts.skipped_already_accepted, 2);
    assert_counts_reconcile(&second.manifest);
}

#[tokio::test]
async fn recovery_rejects_source_kind_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.jpg"), jpeg(1, 1, 1)).unwrap();
    let config = cfg(tmp.path());
    let first = import_folder(&root, &config, Arc::new(InMemoryFrameSink::new()), None)
        .await
        .unwrap();
    let err = import_source(
        IntakeSource::Upload(vec![UploadedFile::new("a.jpg", jpeg(1, 1, 1))]),
        &config,
        Arc::new(InMemoryFrameSink::new()),
        None,
        Some(&RecoveryContext::from_manifest(&first.manifest)),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, TalosError::Validation(_)));
}

#[test]
fn legacy_manifest_without_new_counter_deserializes() {
    let json = r#"{
        "schema_version": "2.0",
        "batch_id": "01J0000000000000000000000",
        "source_summary": { "kind": "folder", "path_or_key": "/in" },
        "counts": {
            "filesystem_entries_seen": 1,
            "image_candidates_discovered": 1,
            "accepted": 1,
            "rejected": 0,
            "skipped_duplicate": 0,
            "failed_sink": 0,
            "metadata_warnings": 0,
            "ignored_unsupported": 0
        },
        "frames": [],
        "errors": [],
        "warnings": []
    }"#;
    let m: BatchManifest = serde_json::from_str(json).unwrap();
    assert_eq!(m.counts.skipped_already_accepted, 0);
    assert!(m.reconcile_ok());
}

// ---------------------------------------------------------------- upload

#[tokio::test]
async fn upload_path_parity_with_folder_path() {
    let tmp = tempfile::tempdir().unwrap();
    let files = vec![
        ("a.jpg", jpeg(10, 0, 0)),
        ("b.JPEG", jpeg(20, 0, 0)),
        ("b_dup.jpg", jpeg(20, 0, 0)),
        ("bad.jpg", b"not-an-image".to_vec()),
        ("readme.txt", b"hello".to_vec()),
    ];
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    for (name, bytes) in &files {
        fs::write(root.join(name), bytes).unwrap();
    }
    let config = cfg(tmp.path());

    let folder_sink = Arc::new(InMemoryFrameSink::new());
    let folder = import_folder(&root, &config, folder_sink.clone(), None)
        .await
        .unwrap();

    let upload_sink = Arc::new(InMemoryFrameSink::new());
    let uploads = files
        .iter()
        .map(|(n, b)| UploadedFile::new(*n, b.clone()))
        .collect();
    let upload = import_upload(uploads, &config, upload_sink.clone(), None)
        .await
        .unwrap();

    assert_eq!(folder.manifest.counts, upload.manifest.counts);
    assert_eq!(upload.manifest.counts.accepted, 2);
    assert_eq!(upload.manifest.counts.skipped_duplicate, 1);
    assert_eq!(upload.manifest.counts.rejected, 1);
    assert_eq!(upload.manifest.counts.ignored_unsupported, 1);
    assert_eq!(folder.outcome, upload.outcome);
    assert_counts_reconcile(&upload.manifest);

    let shas = |envs: Vec<IntakeEnvelope>| -> BTreeSet<String> {
        envs.into_iter().map(|e| e.image.sha256).collect()
    };
    let upload_envs = upload_sink.envelopes().await;
    assert_eq!(
        shas(folder_sink.envelopes().await),
        shas(upload_envs.clone())
    );
    let statuses = |m: &BatchManifest| -> Vec<(String, IntakeStatus)> {
        m.frames
            .iter()
            .map(|f| (f.relative_path.clone(), f.intake_status.clone()))
            .collect()
    };
    assert_eq!(statuses(&folder.manifest), statuses(&upload.manifest));

    assert_eq!(upload.manifest.source_summary.kind, SourceKind::Upload);
    for env in &upload_envs {
        assert_eq!(env.source.kind, SourceKind::Upload);
        // Uploaded bytes persisted unchanged under staging_root/<batch_id>/
        let staged = config
            .staging_root
            .join(upload.manifest.batch_id.as_str())
            .join(&env.source.path_or_key);
        assert_eq!(sha256_hex(&fs::read(&staged).unwrap()), env.image.sha256);
        assert!(env.image.bytes_ref.ends_with(&env.source.path_or_key));
    }
}

#[tokio::test]
async fn upload_path_traversal_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let config = cfg(tmp.path());
    let evil = jpeg(7, 7, 7);
    let files = vec![
        UploadedFile::new("../evil.jpg", evil.clone()),
        UploadedFile::new("/tmp/abs_evil.jpg", evil.clone()),
        UploadedFile::new("sub/evil.jpg", evil.clone()),
        UploadedFile::new("..", evil.clone()),
        UploadedFile::new("x.jpg", evil.clone()).with_relative_dir("../../up"),
        UploadedFile::new("ok.jpg", jpeg(8, 8, 8)).with_relative_dir("cam01"),
    ];
    let sink = Arc::new(InMemoryFrameSink::new());
    let res = import_upload(files, &config, sink.clone(), None)
        .await
        .unwrap();

    let c = &res.manifest.counts;
    assert_eq!(c.filesystem_entries_seen, 6);
    assert_eq!(c.rejected, 5);
    assert_eq!(c.accepted, 1);
    assert_counts_reconcile(&res.manifest);
    assert_eq!(res.outcome, BatchOutcome::PartialFailure);
    for f in res
        .manifest
        .frames
        .iter()
        .filter(|f| f.intake_status != IntakeStatus::Accepted)
    {
        assert_eq!(f.intake_status, IntakeStatus::RejectedValidation);
        assert!(f.error.as_deref().unwrap().contains("unsafe upload"));
    }
    assert_eq!(sink.envelopes().await[0].source.path_or_key, "cam01/ok.jpg");
    assert!(!tmp.path().join("evil.jpg").exists());
    assert!(!tmp.path().join("staging/evil.jpg").exists());
    assert!(!tmp.path().join("up").exists());
    assert!(!Path::new("/tmp/abs_evil.jpg").exists());
}

#[tokio::test]
async fn upload_respects_size_and_count_limits() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = cfg(tmp.path());
    config.max_images_per_batch = 1;
    let small = jpeg(1, 2, 3);
    config.max_image_bytes = small.len() as u64 + 16;
    let files = vec![
        UploadedFile::new("a.jpg", small),
        UploadedFile::new("b.jpg", jpeg(3, 2, 1)),
        UploadedFile::new("huge.jpg", vec![0xFF; config.max_image_bytes as usize + 1]),
    ];
    let res = import_upload(files, &config, Arc::new(InMemoryFrameSink::new()), None)
        .await
        .unwrap();
    assert_eq!(res.manifest.counts.accepted, 1);
    assert_eq!(res.manifest.counts.rejected, 2);
    assert_counts_reconcile(&res.manifest);
    let huge = res
        .manifest
        .frames
        .iter()
        .find(|f| f.relative_path == "huge.jpg")
        .unwrap();
    assert!(huge.error.as_deref().unwrap().contains("max_image_bytes"));
    let staging = config.staging_root.join(res.manifest.batch_id.as_str());
    assert!(!staging.join("huge.jpg").exists(), "oversize never staged");
    let capped = res
        .manifest
        .frames
        .iter()
        .find(|f| f.relative_path == "b.jpg")
        .unwrap();
    assert!(capped
        .error
        .as_deref()
        .unwrap()
        .contains("max_images_per_batch"));
}

// ---------------------------------------------------------------- durable sink

/// Nacks every offer with the given error.
struct NackingDurableSink(fn() -> TalosError);

#[async_trait]
impl DurableSink for NackingDurableSink {
    async fn offer(&self, _e: IntakeEnvelope, _k: IdempotencyKey) -> Result<SinkAck, TalosError> {
        Err((self.0)())
    }
}

/// Stores durably, then loses the ack (returns a transient nack).
struct AckLostDurableSink(Arc<InMemoryDurableSink>);

#[async_trait]
impl DurableSink for AckLostDurableSink {
    async fn offer(&self, e: IntakeEnvelope, k: IdempotencyKey) -> Result<SinkAck, TalosError> {
        self.0.offer(e, k).await?;
        Err(TalosError::Transient("ack lost".into()))
    }
}

#[tokio::test]
async fn durable_sink_nack_maps_to_failed_sink_and_partial_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.jpg"), jpeg(5, 5, 5)).unwrap();
    fs::write(root.join("b.jpg"), jpeg(6, 6, 6)).unwrap();
    let mut config = cfg(tmp.path());
    config.fail_on_empty_batch = false;

    for nack in [
        (|| TalosError::Transient("broker down".into())) as fn() -> TalosError,
        || TalosError::Permanent("poison".into()),
        || TalosError::NotImplemented("no backend".into()),
    ] {
        let sink = Arc::new(DurableSinkAdapter::new(Arc::new(NackingDurableSink(nack))));
        let res = import_folder(&root, &config, sink, None).await.unwrap();
        assert_eq!(res.manifest.counts.failed_sink, 2);
        assert_eq!(res.manifest.counts.accepted, 0);
        assert_eq!(res.outcome, BatchOutcome::PartialFailure);
        assert!(res
            .manifest
            .frames
            .iter()
            .all(|f| f.intake_status == IntakeStatus::FailedSink && f.error.is_some()));
        assert_counts_reconcile(&res.manifest);
    }

    config.fail_batch_on_sink_errors = true;
    let sink = Arc::new(DurableSinkAdapter::new(Arc::new(NackingDurableSink(
        || TalosError::Transient("broker down".into()),
    ))));
    let err = import_folder(&root, &config, sink, None).await.unwrap_err();
    assert!(err.to_string().contains("sink"));
}

#[tokio::test]
async fn durable_sink_accepts_and_dedupes_across_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.jpg"), jpeg(5, 5, 5)).unwrap();
    fs::write(root.join("a_dup.jpg"), jpeg(5, 5, 5)).unwrap();
    let config = cfg(tmp.path());

    let durable = Arc::new(InMemoryDurableSink::new());
    let adapter = Arc::new(DurableSinkAdapter::new(durable.clone()));
    let res = import_folder(&root, &config, adapter.clone(), None)
        .await
        .unwrap();
    assert_eq!(res.manifest.counts.accepted, 1);
    assert_eq!(res.manifest.counts.skipped_duplicate, 1);
    assert_eq!(durable.len().await, 1);
    assert_eq!(adapter.duplicate_acks(), 0);
    let env = &durable.envelopes().await[0];
    assert!(durable.contains(&IdempotencyKey::for_envelope(env)).await);
}

#[tokio::test]
async fn durable_sink_duplicate_key_not_double_counted() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.jpg"), jpeg(11, 11, 11)).unwrap();
    let mut config = cfg(tmp.path());
    config.fail_on_empty_batch = false;

    let durable = Arc::new(InMemoryDurableSink::new());
    // Run 1: the broker stores the frame but the ack is lost → FailedSink.
    let first = import_folder(
        &root,
        &config,
        Arc::new(DurableSinkAdapter::new(Arc::new(AckLostDurableSink(
            durable.clone(),
        )))),
        None,
    )
    .await
    .unwrap();
    assert_eq!(first.manifest.counts.failed_sink, 1);
    assert_eq!(durable.len().await, 1);

    // Run 2 (recovery): same FrameId + sha256 → same key → Duplicate ack, stored once.
    let adapter = Arc::new(DurableSinkAdapter::new(durable.clone()));
    let second = import_source(
        IntakeSource::Folder(root),
        &config,
        adapter.clone(),
        None,
        Some(&RecoveryContext::from_manifest(&first.manifest)),
    )
    .await
    .unwrap();
    assert_eq!(adapter.duplicate_acks(), 1);
    assert_eq!(
        durable.len().await,
        1,
        "durable store must not double-store"
    );
    assert_eq!(second.manifest.counts.accepted, 1);
    assert_eq!(second.manifest.counts.failed_sink, 0);
    assert_eq!(
        second.manifest.frames[0].frame_id,
        first.manifest.frames[0].frame_id
    );
    assert_counts_reconcile(&second.manifest);
    assert_eq!(second.outcome, BatchOutcome::Complete);
}

// ---------------------------------------------------------------- staging plan

#[tokio::test]
async fn plan_cleanup_never_deletes_and_lists_expected_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(root.join("cam")).unwrap();
    fs::write(root.join("cam/good.jpg"), jpeg(1, 1, 1)).unwrap();
    fs::write(root.join("cam/good_dup.jpg"), jpeg(1, 1, 1)).unwrap();
    fs::write(root.join("cam/bad.jpg"), b"not-an-image").unwrap();
    let config = cfg(tmp.path());
    let res = import_folder(&root, &config, Arc::new(InMemoryFrameSink::new()), None)
        .await
        .unwrap();
    let batch_dir = config.staging_root.join(res.manifest.batch_id.as_str());
    let staged = ["cam/good.jpg", "cam/good_dup.jpg", "cam/bad.jpg"].map(|r| batch_dir.join(r));

    let plan = plan_cleanup(&res.manifest, &config.staging_root, StagingPolicy::PlanOnly);
    let mut listed: Vec<_> = plan
        .entries
        .iter()
        .map(|e| (e.path.clone(), e.reason))
        .collect();
    listed.sort_by(|a, b| a.0.cmp(&b.0));
    let mut expected = vec![
        (staged[1].clone(), CleanupReason::SkippedDuplicate),
        (staged[2].clone(), CleanupReason::RejectedPermanent),
    ];
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(listed, expected);
    assert!(!plan.paths().contains(&staged[0].as_path()));

    let retain = plan_cleanup(&res.manifest, &config.staging_root, StagingPolicy::Retain);
    assert!(retain.is_empty());

    let whole = plan_cleanup(
        &res.manifest,
        &config.staging_root,
        StagingPolicy::PlanBatchDir,
    );
    assert_eq!(whole.paths(), vec![batch_dir.as_path()]);
    assert_eq!(whole.entries[0].reason, CleanupReason::BatchDirectory);

    for p in &staged {
        assert!(p.exists(), "plan_cleanup must not delete {}", p.display());
    }
    assert!(batch_dir.is_dir());
    for name in ["cam/good.jpg", "cam/good_dup.jpg", "cam/bad.jpg"] {
        assert!(root.join(name).exists(), "originals untouched");
    }
}

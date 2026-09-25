//! PR #6 remediation regressions: pre-staging resource limits, sink-acknowledged
//! duplicate authority, image-limit classification.

use async_trait::async_trait;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use talos_core::TalosError;
use talos_intake::{
    import_folder, import_source, import_upload, import_zip, BatchManifest, BatchOutcome,
    FailingFrameSink, FrameSink, InMemoryFrameSink, IntakeConfig, IntakeSource, IntakeStatus,
    RecoveryContext, UploadedFile,
};
use talos_types::IntakeEnvelope;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn jpeg_sized(w: u32, h: u32, shade: u8) -> Vec<u8> {
    use image::{ImageBuffer, ImageFormat, Rgb};
    let img = ImageBuffer::from_pixel(w, h, Rgb([shade, shade / 2, 255 - shade]));
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
        .unwrap();
    buf
}

fn jpeg(shade: u8) -> Vec<u8> {
    jpeg_sized(16, 16, shade)
}

fn cfg(dir: &Path) -> IntakeConfig {
    IntakeConfig {
        staging_root: dir.join("staging"),
        max_images_per_batch: 100,
        fail_on_empty_batch: true,
        ..Default::default()
    }
}

fn batch_dir(config: &IntakeConfig, m: &BatchManifest) -> PathBuf {
    config.staging_root.join(m.batch_id.as_str())
}

/// Every file under `dir` (recursive), including hidden partials.
fn staged_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(staged_files(&p));
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn assert_reconciles(m: &BatchManifest) {
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

fn frame<'a>(m: &'a BatchManifest, rel: &str) -> &'a talos_intake::FrameRecord {
    m.frames
        .iter()
        .find(|f| f.relative_path == rel)
        .unwrap_or_else(|| panic!("no frame for {rel}"))
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in entries {
        zip.start_file(*name, opts).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
}

// ------------------------------------------------------------ resource limits

#[tokio::test]
async fn oversized_folder_image_rejected_before_staging() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    let small = jpeg(10);
    let mut config = cfg(tmp.path());
    config.max_image_bytes = small.len() as u64 + 16;
    let big = vec![0xAB; config.max_image_bytes as usize + 1];
    fs::write(root.join("a.jpg"), &small).unwrap();
    fs::write(root.join("big.jpg"), &big).unwrap();

    let res = import_folder(&root, &config, Arc::new(InMemoryFrameSink::new()), None)
        .await
        .unwrap();
    let m = &res.manifest;
    assert_reconciles(m);
    assert_eq!(m.counts.accepted, 1);
    assert_eq!(m.counts.rejected, 1);
    let f = frame(m, "big.jpg");
    assert_eq!(f.intake_status, IntakeStatus::RejectedValidation);
    assert!(f.error.as_deref().unwrap().contains("max_image_bytes"));
    assert!(f.sha256.is_none(), "oversize is never hashed");

    let staged = staged_files(&batch_dir(&config, m));
    assert_eq!(staged, vec![batch_dir(&config, m).join("a.jpg")]);
    assert_eq!(
        fs::read(root.join("big.jpg")).unwrap(),
        big,
        "original untouched"
    );
}

#[tokio::test]
async fn folder_batch_limit_stages_only_admitted_candidates() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    for i in 0..5u8 {
        fs::write(root.join(format!("img_{i}.jpg")), jpeg(20 + i * 30)).unwrap();
    }
    fs::write(root.join("notes.txt"), b"x").unwrap();
    let mut config = cfg(tmp.path());
    config.max_images_per_batch = 2;

    let sink = Arc::new(InMemoryFrameSink::new());
    let res = import_folder(&root, &config, sink.clone(), None)
        .await
        .unwrap();
    let m = &res.manifest;
    assert_reconciles(m);
    assert_eq!(m.counts.image_candidates_discovered, 5);
    assert_eq!(m.counts.accepted, 2);
    assert_eq!(m.counts.rejected, 3);
    assert_eq!(m.counts.ignored_unsupported, 1);
    assert_eq!(sink.len().await, 2);
    for rel in ["img_2.jpg", "img_3.jpg", "img_4.jpg"] {
        let f = frame(m, rel);
        assert_eq!(f.intake_status, IntakeStatus::RejectedValidation);
        assert!(f.error.as_deref().unwrap().contains("max_images_per_batch"));
        assert!(f.sha256.is_none(), "{rel} must not be hashed");
    }
    let dir = batch_dir(&config, m);
    assert_eq!(
        staged_files(&dir),
        vec![dir.join("img_0.jpg"), dir.join("img_1.jpg")],
        "later candidates must not be staged"
    );
}

#[tokio::test]
async fn zip_batch_limit_extracts_only_admitted_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("b.zip");
    let imgs: Vec<Vec<u8>> = (0..5u8).map(|i| jpeg(i * 40)).collect();
    let mut entries: Vec<(String, &[u8])> = imgs
        .iter()
        .enumerate()
        .map(|(i, b)| (format!("cam/{i}.jpg"), b.as_slice()))
        .collect();
    entries.push(("cam/readme.txt".into(), b"not extracted"));
    let refs: Vec<(&str, &[u8])> = entries.iter().map(|(n, b)| (n.as_str(), *b)).collect();
    write_zip(&zip_path, &refs);
    let mut config = cfg(tmp.path());
    config.max_images_per_batch = 2;

    let res = import_zip(&zip_path, &config, Arc::new(InMemoryFrameSink::new()), None)
        .await
        .unwrap();
    let m = &res.manifest;
    assert_reconciles(m);
    assert_eq!(m.counts.filesystem_entries_seen, 6);
    assert_eq!(m.counts.ignored_unsupported, 1);
    assert_eq!(m.counts.accepted, 2);
    assert_eq!(m.counts.rejected, 3);
    let dir = batch_dir(&config, m);
    assert_eq!(
        staged_files(&dir),
        vec![dir.join("cam/0.jpg"), dir.join("cam/1.jpg")]
    );
}

#[tokio::test]
async fn upload_batch_limit_writes_only_admitted_files() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = cfg(tmp.path());
    config.max_images_per_batch = 2;
    let files = (0..4u8)
        .map(|i| UploadedFile::new(format!("u{i}.jpg"), jpeg(i * 50)))
        .collect();
    let res = import_upload(files, &config, Arc::new(InMemoryFrameSink::new()), None)
        .await
        .unwrap();
    assert_reconciles(&res.manifest);
    assert_eq!(res.manifest.counts.accepted, 2);
    assert_eq!(res.manifest.counts.rejected, 2);
    let dir = batch_dir(&config, &res.manifest);
    assert_eq!(
        staged_files(&dir),
        vec![dir.join("u0.jpg"), dir.join("u1.jpg")]
    );
}

#[tokio::test]
async fn zip_oversize_entry_and_repeated_name_rejected_per_frame() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("b.zip");
    let a = jpeg(1);
    let a2 = jpeg(99);
    let mut config = cfg(tmp.path());
    config.max_image_bytes = a.len().max(a2.len()) as u64 + 16;
    let big = vec![0xFF; config.max_image_bytes as usize + 1];
    write_zip(
        &zip_path,
        // `./a.jpg` normalizes to the same staging path as `a.jpg`.
        &[("a.jpg", &a), ("./a.jpg", &a2), ("big.jpg", &big)],
    );

    let res = import_zip(&zip_path, &config, Arc::new(InMemoryFrameSink::new()), None)
        .await
        .unwrap();
    let m = &res.manifest;
    assert_reconciles(m);
    assert_eq!(m.counts.accepted, 1);
    assert_eq!(m.counts.rejected, 2);
    let dir = batch_dir(&config, m);
    assert_eq!(fs::read(dir.join("a.jpg")).unwrap(), a, "first entry kept");
    assert!(!dir.join("big.jpg").exists());
    let rejected: Vec<_> = m
        .frames
        .iter()
        .filter(|f| f.intake_status == IntakeStatus::RejectedValidation)
        .map(|f| f.error.clone().unwrap())
        .collect();
    assert!(rejected
        .iter()
        .any(|e| e.contains("duplicate ZIP entry name")));
    assert!(rejected.iter().any(|e| e.contains("max_image_bytes")));
}

#[tokio::test]
async fn tampered_staged_copy_is_never_overwritten_on_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.jpg"), jpeg(3)).unwrap();
    let mut config = cfg(tmp.path());
    config.fail_on_empty_batch = false;
    let first = import_folder(&root, &config, Arc::new(FailingFrameSink), None)
        .await
        .unwrap();
    let staged = batch_dir(&config, &first.manifest).join("a.jpg");
    fs::write(&staged, b"tampered").unwrap();

    let res = import_source(
        IntakeSource::Folder(root),
        &config,
        Arc::new(InMemoryFrameSink::new()),
        None,
        Some(&RecoveryContext::from_manifest(&first.manifest)),
    )
    .await
    .unwrap();
    assert_eq!(
        frame(&res.manifest, "a.jpg").intake_status,
        IntakeStatus::RejectedPermanent
    );
    assert_eq!(fs::read(&staged).unwrap(), b"tampered");
    assert_reconciles(&res.manifest);
}

// ------------------------------------------------------------ duplicate authority

/// Fails the first `n` submissions, then stores. Deterministic (call-count based).
struct FailFirst {
    n: usize,
    calls: AtomicUsize,
    inner: InMemoryFrameSink,
}

impl FailFirst {
    fn new(n: usize) -> Arc<Self> {
        Arc::new(Self {
            n,
            calls: AtomicUsize::new(0),
            inner: InMemoryFrameSink::new(),
        })
    }
}

#[async_trait]
impl FrameSink for FailFirst {
    async fn submit(&self, envelope: IntakeEnvelope) -> Result<(), TalosError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) < self.n {
            return Err(TalosError::Transient("sink unavailable".into()));
        }
        self.inner.submit(envelope).await
    }
}

#[tokio::test]
async fn sink_failed_first_copy_does_not_suppress_later_same_content() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    let bytes = jpeg(77);
    fs::write(root.join("a.jpg"), &bytes).unwrap();
    fs::write(root.join("b.jpg"), &bytes).unwrap();
    let config = cfg(tmp.path());

    let sink = FailFirst::new(1);
    let res = import_folder(&root, &config, sink.clone(), None)
        .await
        .unwrap();
    let m = &res.manifest;
    assert_reconciles(m);
    assert_eq!(frame(m, "a.jpg").intake_status, IntakeStatus::FailedSink);
    let b = frame(m, "b.jpg");
    assert_eq!(
        b.intake_status,
        IntakeStatus::Accepted,
        "b must not be skipped as a duplicate of a frame the sink never accepted"
    );
    assert_eq!(m.counts.skipped_duplicate, 0);
    assert_eq!(res.outcome, BatchOutcome::PartialFailure);
    let stored = sink.inner.envelopes().await;
    assert_eq!(stored.len(), 1);
    assert_eq!(Some(&stored[0].frame_id), b.frame_id.as_ref());

    // Recovery: content is already durably accepted (as b), so a is a duplicate of b.
    let rerun_sink = Arc::new(InMemoryFrameSink::new());
    let rerun = import_source(
        IntakeSource::Folder(root),
        &config,
        rerun_sink.clone(),
        None,
        Some(&RecoveryContext::from_manifest(m)),
    )
    .await
    .unwrap();
    assert_reconciles(&rerun.manifest);
    assert!(rerun_sink.is_empty().await);
    let a = frame(&rerun.manifest, "a.jpg");
    assert_eq!(a.intake_status, IntakeStatus::SkippedDuplicate);
    assert_eq!(
        Some(&a.duplicate_of.as_ref().unwrap().original_frame_id),
        b.frame_id.as_ref()
    );
    assert_eq!(
        frame(&rerun.manifest, "b.jpg").intake_status,
        IntakeStatus::SkippedAlreadyAccepted
    );
    assert_eq!(rerun.outcome, BatchOutcome::Complete);
}

#[tokio::test]
async fn all_failed_duplicates_retry_with_original_frame_id() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    let bytes = jpeg(88);
    fs::write(root.join("a.jpg"), &bytes).unwrap();
    fs::write(root.join("b.jpg"), &bytes).unwrap();
    let mut config = cfg(tmp.path());
    config.fail_on_empty_batch = false;

    let first = import_folder(&root, &config, Arc::new(FailingFrameSink), None)
        .await
        .unwrap();
    assert_reconciles(&first.manifest);
    assert_eq!(first.manifest.counts.failed_sink, 2);
    assert_eq!(first.manifest.counts.skipped_duplicate, 0);
    let a_id = frame(&first.manifest, "a.jpg").frame_id.clone().unwrap();

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
    assert_reconciles(&second.manifest);
    let envs = sink.envelopes().await;
    assert_eq!(envs.len(), 1);
    assert_eq!(
        envs[0].frame_id, a_id,
        "FailedSink retry reuses its FrameId"
    );
    let b = frame(&second.manifest, "b.jpg");
    assert_eq!(b.intake_status, IntakeStatus::SkippedDuplicate);
    assert_eq!(b.duplicate_of.as_ref().unwrap().original_frame_id, a_id);
    assert_eq!(second.outcome, BatchOutcome::Complete);
}

// ------------------------------------------------------------ image classification

#[tokio::test]
async fn image_limits_and_corruption_map_to_distinct_statuses() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    let ok = jpeg_sized(40, 20, 5);
    let mut truncated = ok.clone();
    truncated.truncate(ok.len() * 2 / 3);
    fs::write(root.join("ok.jpg"), &ok).unwrap();
    fs::write(root.join("wide.jpg"), jpeg_sized(41, 20, 6)).unwrap();
    fs::write(root.join("tall.jpg"), jpeg_sized(40, 21, 7)).unwrap();
    fs::write(root.join("truncated.jpg"), &truncated).unwrap();
    fs::write(root.join("fake.jpg"), b"\xFF\xD8\xFFnot a jpeg").unwrap();
    let mut config = cfg(tmp.path());
    config.max_width = 40;
    config.max_height = 20;

    let res = import_folder(&root, &config, Arc::new(InMemoryFrameSink::new()), None)
        .await
        .unwrap();
    let m = &res.manifest;
    assert_reconciles(m);
    assert_eq!(frame(m, "ok.jpg").intake_status, IntakeStatus::Accepted);
    for rel in ["wide.jpg", "tall.jpg"] {
        assert_eq!(
            frame(m, rel).intake_status,
            IntakeStatus::RejectedValidation
        );
    }
    for rel in ["truncated.jpg", "fake.jpg"] {
        assert_eq!(frame(m, rel).intake_status, IntakeStatus::RejectedPermanent);
    }

    config.max_width = 8192;
    config.max_height = 8192;
    config.max_pixel_count = 799;
    fs::remove_dir_all(&root).unwrap();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ok.jpg"), jpeg_sized(10, 10, 1)).unwrap();
    fs::write(root.join("pixels.jpg"), &ok).unwrap();
    let res = import_folder(&root, &config, Arc::new(InMemoryFrameSink::new()), None)
        .await
        .unwrap();
    assert_eq!(
        frame(&res.manifest, "pixels.jpg").intake_status,
        IntakeStatus::RejectedValidation
    );
}

#[test]
fn import_future_is_send() {
    fn assert_send<T: Send>(_: &T) {}
    let config = IntakeConfig::default();
    let fut = import_source(
        IntakeSource::Zip(PathBuf::from("/nonexistent.zip")),
        &config,
        Arc::new(InMemoryFrameSink::new()),
        None,
        None,
    );
    assert_send(&fut);
}

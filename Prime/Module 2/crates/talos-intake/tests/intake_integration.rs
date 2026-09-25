//! Integration tests for M02 batch & file intake.

use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::sync::Arc;
use talos_core::util::sha256_hex;
use talos_intake::{
    import_folder, import_zip, load_intake_config, BatchOutcome, FailingFrameSink,
    InMemoryFrameSink, IntakeConfig, IntakeStatus,
};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn valid_jpeg() -> Vec<u8> {
    valid_jpeg_with_pixel(RgbPixel(200, 40, 40))
}

fn valid_jpeg_with_pixel(pixel: RgbPixel) -> Vec<u8> {
    use image::{ImageBuffer, ImageFormat, Rgb};
    let img = ImageBuffer::from_pixel(16, 16, Rgb([pixel.0, pixel.1, pixel.2]));
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
        .unwrap();
    buf
}

fn valid_png() -> Vec<u8> {
    use image::{ImageBuffer, ImageFormat, Rgba};
    let img = ImageBuffer::from_pixel(16, 16, Rgba([10u8, 20, 30, 255]));
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .unwrap();
    buf
}

#[derive(Clone, Copy)]
struct RgbPixel(u8, u8, u8);

fn cfg_with_staging(dir: &std::path::Path) -> IntakeConfig {
    IntakeConfig {
        staging_root: dir.join("staging"),
        max_images_per_batch: 100,
        fail_on_empty_batch: true,
        ..Default::default()
    }
}

#[tokio::test]
async fn accepts_jpg_jpeg_png() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(root.join("cam")).unwrap();
    let jpeg_a = valid_jpeg_with_pixel(RgbPixel(1, 2, 3));
    let jpeg_b = valid_jpeg_with_pixel(RgbPixel(4, 5, 6));
    fs::write(root.join("cam/a.JPG"), jpeg_a).unwrap();
    fs::write(root.join("cam/b.jpeg"), jpeg_b).unwrap();
    fs::write(root.join("cam/c.png"), valid_png()).unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_folder(&root, &cfg, sink.clone(), None)
        .await
        .unwrap();
    assert_eq!(res.manifest.counts.accepted, 3);
    assert_eq!(res.outcome, BatchOutcome::Complete);
    assert!(res.manifest.reconcile_ok());
    assert_eq!(sink.len().await, 3);
}

#[tokio::test]
async fn unsupported_file_classified_not_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("note.txt"), b"hello").unwrap();
    fs::write(root.join("ok.jpg"), valid_jpeg()).unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_folder(&root, &cfg, sink.clone(), None)
        .await
        .unwrap();
    assert_eq!(res.manifest.counts.ignored_unsupported, 1);
    assert_eq!(res.manifest.counts.accepted, 1);
    assert_eq!(res.manifest.counts.image_candidates_discovered, 1);
    assert_eq!(res.manifest.counts.filesystem_entries_seen, 2);
    assert!(res.manifest.reconcile_ok());
}

#[tokio::test]
async fn corrupt_image_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("bad.jpg"), b"not-really-jpeg").unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let err = import_folder(&root, &cfg, sink, None).await.unwrap_err();
    assert!(err.to_string().contains("empty batch") || err.to_string().contains("corrupt"));
}

#[tokio::test]
async fn fake_magic_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    let mut fake = vec![0xFF, 0xD8, 0xFF, 0xE0];
    fake.extend_from_slice(b"JFIF\0not-a-real-jpeg-body");
    fs::write(root.join("fake.jpg"), &fake).unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let mut cfg = cfg_with_staging(tmp.path());
    cfg.fail_on_empty_batch = false;
    let res = import_folder(&root, &cfg, sink, None).await.unwrap();
    assert_eq!(res.manifest.counts.rejected, 1);
    assert_eq!(
        res.manifest.frames[0].intake_status,
        IntakeStatus::RejectedPermanent
    );
    assert_eq!(res.outcome, BatchOutcome::PartialFailure);
}

#[tokio::test]
async fn truncated_image_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    let mut bytes = valid_jpeg();
    bytes.truncate(bytes.len() / 3);
    fs::write(root.join("trunc.jpg"), &bytes).unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let mut cfg = cfg_with_staging(tmp.path());
    cfg.fail_on_empty_batch = false;
    let res = import_folder(&root, &cfg, sink, None).await.unwrap();
    assert_eq!(res.manifest.counts.rejected, 1);
    assert_eq!(
        res.manifest.frames[0].intake_status,
        IntakeStatus::RejectedPermanent
    );
}

#[tokio::test]
async fn accounting_invariants() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(root.join("cam")).unwrap();
    fs::write(
        root.join("cam/a.jpg"),
        valid_jpeg_with_pixel(RgbPixel(1, 0, 0)),
    )
    .unwrap();
    fs::write(
        root.join("cam/b.jpg"),
        valid_jpeg_with_pixel(RgbPixel(2, 0, 0)),
    )
    .unwrap();
    fs::write(root.join("readme.txt"), b"notes").unwrap();
    fs::write(root.join("meta.csv"), b"relative_path\n").unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_folder(&root, &cfg, sink, None).await.unwrap();
    let c = &res.manifest.counts;
    assert_eq!(c.filesystem_entries_seen, 4);
    assert_eq!(c.image_candidates_discovered, 2);
    assert_eq!(c.ignored_unsupported, 2);
    assert_eq!(
        c.image_candidates_discovered,
        c.accepted + c.rejected + c.skipped_duplicate + c.failed_sink
    );
    assert_eq!(
        c.filesystem_entries_seen,
        c.image_candidates_discovered + c.ignored_unsupported
    );
    assert!(res.manifest.reconcile_ok());
}

#[tokio::test]
async fn deterministic_traversal_order() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(root.join("z")).unwrap();
    fs::create_dir_all(root.join("a")).unwrap();
    fs::write(
        root.join("a/1.jpg"),
        valid_jpeg_with_pixel(RgbPixel(10, 0, 0)),
    )
    .unwrap();
    fs::write(
        root.join("z/2.jpg"),
        valid_jpeg_with_pixel(RgbPixel(20, 0, 0)),
    )
    .unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_folder(&root, &cfg, sink, None).await.unwrap();
    let paths: Vec<_> = res
        .manifest
        .frames
        .iter()
        .filter(|f| f.intake_status == IntakeStatus::Accepted)
        .map(|f| f.relative_path.as_str())
        .collect();
    assert_eq!(paths, vec!["a/1.jpg", "z/2.jpg"]);
}

#[tokio::test]
async fn sha256_and_bytes_immutable() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    let bytes = valid_jpeg();
    let expected = sha256_hex(&bytes);
    fs::write(root.join("x.jpg"), &bytes).unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_folder(&root, &cfg, sink.clone(), None)
        .await
        .unwrap();
    let env = sink.envelopes().await;
    assert_eq!(env[0].image.sha256, expected);
    // Original file unchanged
    let after = fs::read(root.join("x.jpg")).unwrap();
    assert_eq!(after, bytes);
    assert_eq!(sha256_hex(&after), expected);
    assert_eq!(res.outcome, BatchOutcome::Complete);
}

#[tokio::test]
async fn duplicate_references_original_frame_id() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(root.join("a")).unwrap();
    fs::create_dir_all(root.join("b")).unwrap();
    let bytes = valid_jpeg();
    fs::write(root.join("a/one.jpg"), &bytes).unwrap();
    fs::write(root.join("b/two.jpg"), &bytes).unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_folder(&root, &cfg, sink.clone(), None)
        .await
        .unwrap();
    assert_eq!(res.manifest.counts.accepted, 1);
    assert_eq!(res.manifest.counts.skipped_duplicate, 1);
    assert_eq!(sink.len().await, 1);
    assert_eq!(res.outcome, BatchOutcome::Complete);

    let accepted = res
        .manifest
        .frames
        .iter()
        .find(|f| f.intake_status == IntakeStatus::Accepted)
        .unwrap();
    let dup = res
        .manifest
        .frames
        .iter()
        .find(|f| f.intake_status == IntakeStatus::SkippedDuplicate)
        .unwrap();
    let orig = accepted.frame_id.as_ref().unwrap();
    assert_eq!(dup.duplicate_of.as_ref().unwrap().original_frame_id, *orig);
    assert!(res.manifest.reconcile_ok());
}

#[tokio::test]
async fn csv_metadata_join_by_relative_path() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(root.join("cam01")).unwrap();
    fs::write(root.join("cam01/img_0001.jpg"), valid_jpeg()).unwrap();
    let csv_path = tmp.path().join("meta.csv");
    let mut f = fs::File::create(&csv_path).unwrap();
    writeln!(
        f,
        "relative_path,camera_id,captured_at,location\ncam01/img_0001.jpg,CAM-01,2026-09-22T05:00:00Z,Raipur"
    )
    .unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_folder(&root, &cfg, sink.clone(), Some(&csv_path))
        .await
        .unwrap();
    let env = &sink.envelopes().await[0];
    assert_eq!(env.metadata.camera_id.as_ref().unwrap().as_str(), "CAM-01");
    assert_eq!(env.metadata.location.as_deref(), Some("Raipur"));
    // Sidecar outside root is not counted in filesystem_entries_seen
    assert_eq!(res.manifest.counts.filesystem_entries_seen, 1);
    assert!(res.manifest.reconcile_ok());
}

#[tokio::test]
async fn basename_fallback_and_ambiguous() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(root.join("a")).unwrap();
    fs::create_dir_all(root.join("b")).unwrap();
    let j1 = valid_jpeg_with_pixel(RgbPixel(1, 1, 1));
    let j2 = valid_jpeg_with_pixel(RgbPixel(2, 2, 2));
    fs::write(root.join("a/same.jpg"), &j1).unwrap();
    fs::write(root.join("b/same.jpg"), &j2).unwrap();

    // Metadata only by basename "same.jpg" — ambiguous
    let csv_path = tmp.path().join("meta.csv");
    let mut f = fs::File::create(&csv_path).unwrap();
    writeln!(f, "file,camera_id\nsame.jpg,CAM-X").unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_folder(&root, &cfg, sink.clone(), Some(&csv_path))
        .await
        .unwrap();
    for env in sink.envelopes().await {
        assert!(env.metadata.camera_id.is_none());
    }
    assert!(res
        .manifest
        .warnings
        .iter()
        .any(|w| w.contains("ambiguous")));
}

#[tokio::test]
async fn malformed_metadata_row_warns() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ok.jpg"), valid_jpeg()).unwrap();
    let csv_path = tmp.path().join("meta.csv");
    let mut f = fs::File::create(&csv_path).unwrap();
    writeln!(f, "relative_path,captured_at\nok.jpg,NOT-A-TIMESTAMP\n").unwrap();

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_folder(&root, &cfg, sink, Some(&csv_path))
        .await
        .unwrap();
    assert!(res.manifest.counts.metadata_warnings >= 1);
    assert_eq!(res.manifest.counts.accepted, 1);
}

#[tokio::test]
async fn missing_metadata_ok() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ok.jpg"), valid_jpeg()).unwrap();
    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_folder(&root, &cfg, sink.clone(), None)
        .await
        .unwrap();
    assert!(sink.envelopes().await[0].metadata.camera_id.is_none());
    assert_eq!(res.manifest.counts.accepted, 1);
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

#[tokio::test]
async fn zip_import_ok() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("b.zip");
    let jpeg = valid_jpeg();
    write_zip(&zip_path, &[("cam/x.jpg", &jpeg)]);

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let res = import_zip(&zip_path, &cfg, sink.clone(), None)
        .await
        .unwrap();
    assert_eq!(res.manifest.counts.accepted, 1);
    assert_eq!(sink.len().await, 1);
}

#[tokio::test]
async fn zip_slip_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("evil.zip");
    let jpeg = valid_jpeg();
    write_zip(&zip_path, &[("../escape.jpg", &jpeg)]);

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let err = import_zip(&zip_path, &cfg, sink, None).await.unwrap_err();
    assert!(
        err.to_string().to_ascii_lowercase().contains("slip")
            || err.to_string().contains("..")
            || err.to_string().contains("rejected")
    );
}

#[tokio::test]
async fn zip_absolute_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let zip_path = tmp.path().join("abs.zip");
    let jpeg = valid_jpeg();
    write_zip(&zip_path, &[("/tmp/abs.jpg", &jpeg)]);

    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let err = import_zip(&zip_path, &cfg, sink, None).await.unwrap_err();
    assert!(err.to_string().contains("absolute") || err.to_string().contains("rejected"));
}

#[tokio::test]
async fn sink_failure_partial_failure_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ok.jpg"), valid_jpeg()).unwrap();

    let sink = Arc::new(FailingFrameSink);
    let mut cfg = cfg_with_staging(tmp.path());
    cfg.fail_on_empty_batch = false;
    cfg.fail_batch_on_sink_errors = false;
    let res = import_folder(&root, &cfg, sink, None).await.unwrap();
    assert_eq!(res.manifest.counts.failed_sink, 1);
    assert_eq!(res.manifest.counts.accepted, 0);
    assert!(res.manifest.reconcile_ok());
    assert_eq!(res.outcome, BatchOutcome::PartialFailure);
    assert_eq!(
        res.manifest.frames[0].intake_status,
        IntakeStatus::FailedSink
    );
}

#[tokio::test]
async fn sink_failure_fails_batch_when_configured() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ok.jpg"), valid_jpeg()).unwrap();

    let sink = Arc::new(FailingFrameSink);
    let mut cfg = cfg_with_staging(tmp.path());
    cfg.fail_on_empty_batch = false;
    cfg.fail_batch_on_sink_errors = true;
    let err = import_folder(&root, &cfg, sink, None).await.unwrap_err();
    assert!(err.to_string().contains("sink"));
}

#[tokio::test]
async fn envelope_schema_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("batch");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ok.jpg"), valid_jpeg()).unwrap();
    let sink = Arc::new(InMemoryFrameSink::new());
    let cfg = cfg_with_staging(tmp.path());
    let _ = import_folder(&root, &cfg, sink.clone(), None)
        .await
        .unwrap();
    let env = &sink.envelopes().await[0];
    assert_eq!(env.schema_version, "2.0");
    let json = serde_json::to_string(env).unwrap();
    let back: talos_types::IntakeEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(back.schema_version, "2.0");
}

#[test]
fn loads_repo_intake_toml() {
    // CARGO_MANIFEST_DIR = Prime/Module 2/crates/talos-intake → ../../configs
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs");
    let cfg = load_intake_config(Some(&path)).unwrap();
    assert_eq!(cfg.max_images_per_batch, 10_000);
    assert_eq!(cfg.max_width, 8192);
    assert_eq!(cfg.max_height, 8192);
    assert_eq!(cfg.max_pixel_count, 25_000_000);
    assert!(!cfg.fail_batch_on_sink_errors);
    assert_eq!(cfg.staging_policy, talos_intake::StagingPolicy::Retain);
}

#[test]
fn batch_counts_deserializes_discovered_alias() {
    let json = r#"{
        "filesystem_entries_seen": 2,
        "discovered": 1,
        "accepted": 1,
        "rejected": 0,
        "skipped_duplicate": 0,
        "failed_sink": 0,
        "metadata_warnings": 0,
        "ignored_unsupported": 1
    }"#;
    let counts: talos_intake::BatchCounts = serde_json::from_str(json).unwrap();
    assert_eq!(counts.image_candidates_discovered, 1);
}

use crate::config::IntakeConfig;
use crate::discover::{basename_of, discover_images, DiscoveredFile};
use crate::image::validate_image;
use crate::metadata::MetadataIndex;
use crate::sink::FrameSink;
use crate::types::{
    BatchCounts, BatchManifest, BatchOutcome, DuplicateReference, FrameRecord, ImportRequest,
    IntakeStatus,
};
use crate::zip_safe::{extract_zip_safe, read_file_capped};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use talos_core::util::{new_batch_id, new_frame_id, sha256_hex};
use talos_core::TalosError;
use talos_types::{ImageRef, IntakeEnvelope, SourceKind, SourceRef, SCHEMA_VERSION};
use tracing::{info, warn};

#[derive(Clone, Debug)]
pub struct ImportResult {
    pub manifest: BatchManifest,
    pub outcome: BatchOutcome,
}

/// Import a folder of images (no continuous watching).
pub async fn import_folder(
    root: &Path,
    config: &IntakeConfig,
    sink: Arc<dyn FrameSink>,
    metadata_path: Option<&Path>,
) -> Result<ImportResult, TalosError> {
    let batch_id = new_batch_id();
    let request = ImportRequest {
        source_kind: SourceKind::Folder,
        path_or_key: root.display().to_string(),
        metadata_path: metadata_path.map(|p| p.to_path_buf()),
    };
    let staging = config.staging_root.join(batch_id.as_str());
    std::fs::create_dir_all(&staging).map_err(|e| TalosError::Transient(e.to_string()))?;

    // Stage by copying originals (bytes unchanged)
    let (candidates, ignored, filesystem_entries_seen) =
        discover_images(root, config).map_err(TalosError::Validation)?;
    let staged = stage_copies(&candidates, root, &staging)?;

    run_batch(
        batch_id,
        request,
        &staging,
        staged,
        ignored,
        filesystem_entries_seen,
        config,
        sink,
        metadata_path,
        SourceKind::Folder,
    )
    .await
}

/// Import a ZIP archive with zip-slip protection.
pub async fn import_zip(
    zip_path: &Path,
    config: &IntakeConfig,
    sink: Arc<dyn FrameSink>,
    metadata_path: Option<&Path>,
) -> Result<ImportResult, TalosError> {
    let batch_id = new_batch_id();
    let request = ImportRequest {
        source_kind: SourceKind::Zip,
        path_or_key: zip_path.display().to_string(),
        metadata_path: metadata_path.map(|p| p.to_path_buf()),
    };
    let staging = config.staging_root.join(batch_id.as_str());
    extract_zip_safe(zip_path, &staging, config)?;

    let (candidates, ignored, filesystem_entries_seen) =
        discover_images(&staging, config).map_err(TalosError::Validation)?;
    // Already in staging; use as-is
    let staged: Vec<DiscoveredFile> = candidates;

    run_batch(
        batch_id,
        request,
        &staging,
        staged,
        ignored,
        filesystem_entries_seen,
        config,
        sink,
        metadata_path,
        SourceKind::Zip,
    )
    .await
}

fn stage_copies(
    candidates: &[DiscoveredFile],
    root: &Path,
    staging: &Path,
) -> Result<Vec<DiscoveredFile>, TalosError> {
    let mut out = Vec::with_capacity(candidates.len());
    for c in candidates {
        let dest = staging.join(&c.relative);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| TalosError::Transient(e.to_string()))?;
        }
        // Byte-preserving copy
        std::fs::copy(&c.absolute, &dest).map_err(|e| {
            TalosError::Transient(format!(
                "copy {} → {}: {e}",
                c.absolute.display(),
                dest.display()
            ))
        })?;
        // Verify immutability of staging vs source
        let src_bytes =
            std::fs::read(&c.absolute).map_err(|e| TalosError::Permanent(e.to_string()))?;
        let dst_bytes = std::fs::read(&dest).map_err(|e| TalosError::Permanent(e.to_string()))?;
        if src_bytes != dst_bytes {
            return Err(TalosError::Internal(
                "staged bytes differ from original — immutability violated".into(),
            ));
        }
        let _ = root; // relative already computed from root
        out.push(DiscoveredFile {
            absolute: dest,
            relative: c.relative.clone(),
        });
    }
    Ok(out)
}

fn batch_outcome(counts: &BatchCounts) -> BatchOutcome {
    if counts.failed_sink > 0 || counts.rejected > 0 {
        BatchOutcome::PartialFailure
    } else {
        BatchOutcome::Complete
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_batch(
    batch_id: talos_types::BatchId,
    request: ImportRequest,
    staging: &Path,
    staged: Vec<DiscoveredFile>,
    ignored_unsupported: u32,
    filesystem_entries_seen: u32,
    config: &IntakeConfig,
    sink: Arc<dyn FrameSink>,
    metadata_path: Option<&Path>,
    source_kind: SourceKind,
) -> Result<ImportResult, TalosError> {
    let _span = tracing::info_span!("batch.run", batch_id = %batch_id).entered();
    info!(%batch_id, "batch_started");

    let mut meta_index = if let Some(mp) = metadata_path {
        Some(MetadataIndex::load(mp)?)
    } else {
        None
    };

    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut metadata_warnings = 0u32;
    if let Some(ref idx) = meta_index {
        metadata_warnings = idx.row_errors;
        warnings.extend(idx.warnings.clone());
        if config.fail_batch_on_metadata_errors && idx.row_errors > 0 {
            return Err(TalosError::Validation(format!(
                "metadata errors ({}) and fail_batch_on_metadata_errors=true",
                idx.row_errors
            )));
        }
    }

    let image_candidates_discovered = staged.len() as u32;
    let mut frames = Vec::new();
    let mut seen_sha: HashMap<String, talos_types::FrameId> = HashMap::new();
    let mut counts = BatchCounts {
        filesystem_entries_seen,
        image_candidates_discovered,
        ignored_unsupported,
        ..BatchCounts::default()
    };

    let mut basename_counts: HashMap<String, u32> = HashMap::new();
    for file in &staged {
        *basename_counts
            .entry(basename_of(&file.relative))
            .or_default() += 1;
    }

    let mut accepted_unique = 0u32;

    for file in &staged {
        info!(relative_path = %file.relative, "file_discovered");

        if accepted_unique >= config.max_images_per_batch {
            frames.push(FrameRecord {
                frame_id: None,
                relative_path: file.relative.clone(),
                sha256: None,
                intake_status: IntakeStatus::RejectedValidation,
                duplicate_of: None,
                envelope: None,
                error: Some("max_images_per_batch exceeded".into()),
            });
            counts.rejected = counts.rejected.saturating_add(1);
            continue;
        }

        let bytes = match read_file_capped(&file.absolute, config.max_image_bytes) {
            Ok(b) => b,
            Err(e) => {
                let status = match &e {
                    TalosError::Permanent(_) => IntakeStatus::RejectedPermanent,
                    _ => IntakeStatus::RejectedValidation,
                };
                let msg = e.to_string();
                warn!(error = %msg, relative_path = %file.relative, "file_rejected");
                frames.push(FrameRecord {
                    frame_id: None,
                    relative_path: file.relative.clone(),
                    sha256: None,
                    intake_status: status,
                    duplicate_of: None,
                    envelope: None,
                    error: Some(msg),
                });
                counts.rejected = counts.rejected.saturating_add(1);
                errors.push(format!("{}: {}", file.relative, e));
                continue;
            }
        };

        let content_type = match validate_image(&bytes, config) {
            Ok(ct) => ct,
            Err(e) => {
                let status = match &e {
                    TalosError::Permanent(_) => IntakeStatus::RejectedPermanent,
                    _ => IntakeStatus::RejectedValidation,
                };
                let msg = e.to_string();
                warn!(error = %msg, relative_path = %file.relative, "file_rejected");
                frames.push(FrameRecord {
                    frame_id: None,
                    relative_path: file.relative.clone(),
                    sha256: None,
                    intake_status: status,
                    duplicate_of: None,
                    envelope: None,
                    error: Some(msg.clone()),
                });
                counts.rejected = counts.rejected.saturating_add(1);
                errors.push(format!("{}: {msg}", file.relative));
                continue;
            }
        };

        // SHA-256 over ORIGINAL accepted bytes (no mutation / re-encode)
        let sha = sha256_hex(&bytes);
        // Prove immutability: re-read file and compare
        let again =
            std::fs::read(&file.absolute).map_err(|e| TalosError::Permanent(e.to_string()))?;
        if again != bytes {
            return Err(TalosError::Internal(
                "file bytes changed during intake".into(),
            ));
        }
        if sha256_hex(&again) != sha {
            return Err(TalosError::Internal("SHA-256 mismatch on re-hash".into()));
        }

        if let Some(orig) = seen_sha.get(&sha) {
            info!(relative_path = %file.relative, %sha, "duplicate_detected");
            frames.push(FrameRecord {
                frame_id: None,
                relative_path: file.relative.clone(),
                sha256: Some(sha.clone()),
                intake_status: IntakeStatus::SkippedDuplicate,
                duplicate_of: Some(DuplicateReference {
                    original_frame_id: orig.clone(),
                    sha256: sha,
                }),
                envelope: None,
                error: None,
            });
            counts.skipped_duplicate = counts.skipped_duplicate.saturating_add(1);
            continue;
        }

        let frame_id = new_frame_id();
        seen_sha.insert(sha.clone(), frame_id.clone());

        let mut metadata = talos_types::FrameMetadata::default();
        if let Some(ref mut idx) = meta_index {
            let base_count = basename_counts
                .get(&basename_of(&file.relative))
                .copied()
                .unwrap_or(1);
            let (md, warn) = idx.join(&file.relative, base_count);
            metadata = md;
            if let Some(w) = warn {
                warnings.push(w.clone());
                metadata_warnings = metadata_warnings.saturating_add(1);
            }
        }

        let bytes_ref = format!("file://{}", file.absolute.display());
        let mut envelope = IntakeEnvelope::new(
            batch_id.clone(),
            frame_id.clone(),
            SourceRef {
                kind: source_kind.clone(),
                path_or_key: file.relative.clone(),
            },
            ImageRef {
                content_type: content_type.to_string(),
                sha256: sha.clone(),
                bytes_ref,
            },
        );
        envelope.metadata = metadata;

        match sink.submit(envelope.clone()).await {
            Ok(()) => {
                info!(frame_id = %frame_id, relative_path = %file.relative, "frame_accepted");
                accepted_unique = accepted_unique.saturating_add(1);
                counts.accepted = counts.accepted.saturating_add(1);
                frames.push(FrameRecord {
                    frame_id: Some(frame_id),
                    relative_path: file.relative.clone(),
                    sha256: Some(sha),
                    intake_status: IntakeStatus::Accepted,
                    duplicate_of: None,
                    envelope: Some(envelope),
                    error: None,
                });
            }
            Err(e) => {
                warn!(error = %e, relative_path = %file.relative, "sink_failure");
                counts.failed_sink = counts.failed_sink.saturating_add(1);
                frames.push(FrameRecord {
                    frame_id: Some(frame_id),
                    relative_path: file.relative.clone(),
                    sha256: Some(sha),
                    intake_status: IntakeStatus::FailedSink,
                    duplicate_of: None,
                    envelope: Some(envelope),
                    error: Some(e.to_string()),
                });
                errors.push(format!("sink {}: {e}", file.relative));
            }
        }
    }

    counts.metadata_warnings = metadata_warnings;

    // Prefer Err for empty batches when fail_on_empty_batch (Rejected at API level).
    if counts.accepted == 0 && config.fail_on_empty_batch {
        return Err(TalosError::Validation(
            "empty batch: no accepted images".into(),
        ));
    }

    if counts.failed_sink > 0 && config.fail_batch_on_sink_errors {
        return Err(TalosError::Transient(format!(
            "batch rejected: {} sink failures and fail_batch_on_sink_errors=true",
            counts.failed_sink
        )));
    }

    let manifest = BatchManifest {
        schema_version: SCHEMA_VERSION.to_owned(),
        batch_id: batch_id.clone(),
        source_summary: SourceRef {
            kind: source_kind,
            path_or_key: request.path_or_key,
        },
        counts: counts.clone(),
        frames,
        errors,
        warnings,
    };

    if !manifest.reconcile_ok() {
        return Err(TalosError::Internal(format!(
            "manifest reconciliation failed: filesystem_entries_seen={} image_candidates_discovered={} accepted={} rejected={} dup={} sink={} ignored={}",
            manifest.counts.filesystem_entries_seen,
            manifest.counts.image_candidates_discovered,
            manifest.counts.accepted,
            manifest.counts.rejected,
            manifest.counts.skipped_duplicate,
            manifest.counts.failed_sink,
            manifest.counts.ignored_unsupported
        )));
    }

    let outcome = batch_outcome(&counts);
    info!(%batch_id, ?outcome, "batch_completed");
    let _ = staging;
    // Staging under staging_root/batch_id is owned by M02 for intake; cleanup is
    // deferred to the orchestrator / M10 (not deleted here).
    Ok(ImportResult { manifest, outcome })
}

use crate::config::IntakeConfig;
use crate::discover::{basename_of, discover_images, DiscoveredFile};
use crate::image::validate_image;
use crate::metadata::MetadataIndex;
use crate::recovery::{RecoveryContext, RecoveryKey};
use crate::sink::FrameSink;
use crate::staging::stage_file;
use crate::types::{
    BatchCounts, BatchManifest, BatchOutcome, DuplicateReference, FrameRecord, IntakeStatus,
};
use crate::upload::{stage_uploads, UploadedFile};
use crate::zip_safe::{extract_zip_safe, read_file_capped};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use talos_core::util::{new_batch_id, new_frame_id, sha256_hex};
use talos_core::TalosError;
use talos_types::{BatchId, ImageRef, IntakeEnvelope, SourceKind, SourceRef, SCHEMA_VERSION};
use tracing::{info, warn};

#[derive(Clone, Debug)]
pub struct ImportResult {
    pub manifest: BatchManifest,
    pub outcome: BatchOutcome,
}

/// Any supported batch input. All variants share validation, hashing, dedupe, metadata
/// join, sink hand-off and manifest accounting.
#[derive(Clone, Debug)]
pub enum IntakeSource {
    Folder(PathBuf),
    Zip(PathBuf),
    Upload(Vec<UploadedFile>),
}

impl IntakeSource {
    fn kind(&self) -> SourceKind {
        match self {
            Self::Folder(_) => SourceKind::Folder,
            Self::Zip(_) => SourceKind::Zip,
            Self::Upload(_) => SourceKind::Upload,
        }
    }
}

/// An image candidate after staging (or rejected before it could be staged).
pub(crate) struct Candidate {
    pub relative: String,
    pub state: CandidateState,
}

pub(crate) enum CandidateState {
    Staged(PathBuf),
    Rejected(TalosError),
}

/// Import a folder of images (no continuous watching).
pub async fn import_folder(
    root: &Path,
    config: &IntakeConfig,
    sink: Arc<dyn FrameSink>,
    metadata_path: Option<&Path>,
) -> Result<ImportResult, TalosError> {
    import_source(
        IntakeSource::Folder(root.to_path_buf()),
        config,
        sink,
        metadata_path,
        None,
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
    import_source(
        IntakeSource::Zip(zip_path.to_path_buf()),
        config,
        sink,
        metadata_path,
        None,
    )
    .await
}

/// Import in-memory uploaded files (library surface; the HTTP route is M12).
/// Bytes are persisted unchanged to `staging_root/<batch_id>/`.
pub async fn import_upload(
    files: Vec<UploadedFile>,
    config: &IntakeConfig,
    sink: Arc<dyn FrameSink>,
    metadata_path: Option<&Path>,
) -> Result<ImportResult, TalosError> {
    import_source(
        IntakeSource::Upload(files),
        config,
        sink,
        metadata_path,
        None,
    )
    .await
}

/// General entry point. With `recovery = Some(ctx)` the run reuses `ctx.batch_id()` and
/// its staging directory, skips already-accepted `(relative_path, sha256)` pairs
/// (`skipped_already_accepted`, never re-emitted) and retries prior `FailedSink` frames
/// with their original `FrameId`. The returned manifest describes this run only.
pub async fn import_source(
    source: IntakeSource,
    config: &IntakeConfig,
    sink: Arc<dyn FrameSink>,
    metadata_path: Option<&Path>,
    recovery: Option<&RecoveryContext>,
) -> Result<ImportResult, TalosError> {
    let source_kind = source.kind();
    let batch_id = match recovery {
        Some(ctx) => {
            if *ctx.source_kind() != source_kind {
                return Err(TalosError::Validation(format!(
                    "recovery source kind mismatch: prior {:?}, now {:?}",
                    ctx.source_kind(),
                    source_kind
                )));
            }
            ctx.batch_id().clone()
        }
        None => new_batch_id(),
    };
    let staging = config.staging_root.join(batch_id.as_str());

    let (path_or_key, candidates, ignored, filesystem_entries_seen) = match source {
        IntakeSource::Folder(root) => {
            std::fs::create_dir_all(&staging).map_err(|e| TalosError::Transient(e.to_string()))?;
            let (found, ignored, seen) =
                discover_images(&root, config).map_err(TalosError::Validation)?;
            let staged = stage_copies(&found, &staging)?;
            (root.display().to_string(), staged, ignored, seen)
        }
        IntakeSource::Zip(zip_path) => {
            extract_zip_safe(&zip_path, &staging, config)?;
            let (found, ignored, seen) =
                discover_images(&staging, config).map_err(TalosError::Validation)?;
            (
                zip_path.display().to_string(),
                found.into_iter().map(staged_candidate).collect(),
                ignored,
                seen,
            )
        }
        IntakeSource::Upload(files) => {
            std::fs::create_dir_all(&staging).map_err(|e| TalosError::Transient(e.to_string()))?;
            let (candidates, ignored, seen) = stage_uploads(files, &staging, config)?;
            ("upload".to_owned(), candidates, ignored, seen)
        }
    };

    run_batch(
        BatchSetup {
            batch_id,
            source_kind,
            path_or_key,
            ignored_unsupported: ignored,
            filesystem_entries_seen,
        },
        candidates,
        config,
        sink,
        metadata_path,
        recovery,
    )
    .await
}

fn staged_candidate(file: DiscoveredFile) -> Candidate {
    Candidate {
        relative: file.relative,
        state: CandidateState::Staged(file.absolute),
    }
}

fn stage_copies(
    candidates: &[DiscoveredFile],
    staging: &Path,
) -> Result<Vec<Candidate>, TalosError> {
    let mut out = Vec::with_capacity(candidates.len());
    for c in candidates {
        let dest = staging.join(&c.relative);
        stage_file(&c.absolute, &dest)?;
        out.push(Candidate {
            relative: c.relative.clone(),
            state: CandidateState::Staged(dest),
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

struct BatchSetup {
    batch_id: BatchId,
    source_kind: SourceKind,
    path_or_key: String,
    ignored_unsupported: u32,
    filesystem_entries_seen: u32,
}

fn rejected_record(relative: &str, err: &TalosError) -> FrameRecord {
    let status = match err {
        TalosError::Permanent(_) => IntakeStatus::RejectedPermanent,
        _ => IntakeStatus::RejectedValidation,
    };
    FrameRecord {
        frame_id: None,
        relative_path: relative.to_owned(),
        sha256: None,
        intake_status: status,
        duplicate_of: None,
        envelope: None,
        error: Some(err.to_string()),
    }
}

async fn run_batch(
    setup: BatchSetup,
    candidates: Vec<Candidate>,
    config: &IntakeConfig,
    sink: Arc<dyn FrameSink>,
    metadata_path: Option<&Path>,
    recovery: Option<&RecoveryContext>,
) -> Result<ImportResult, TalosError> {
    let BatchSetup {
        batch_id,
        source_kind,
        path_or_key,
        ignored_unsupported,
        filesystem_entries_seen,
    } = setup;
    let _span = tracing::info_span!("batch.run", batch_id = %batch_id).entered();
    info!(%batch_id, recovery = recovery.is_some(), "batch_started");

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

    let image_candidates_discovered = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
    let mut frames = Vec::new();
    let mut seen_sha: HashMap<String, talos_types::FrameId> = recovery
        .map(RecoveryContext::accepted_by_sha)
        .unwrap_or_default();
    let mut counts = BatchCounts {
        filesystem_entries_seen,
        image_candidates_discovered,
        ignored_unsupported,
        ..BatchCounts::default()
    };

    let mut basename_counts: HashMap<String, u32> = HashMap::new();
    for c in &candidates {
        *basename_counts.entry(basename_of(&c.relative)).or_default() += 1;
    }

    // Accepted in this run + already accepted by a prior run of this batch.
    let mut accepted_unique = 0u32;

    for candidate in &candidates {
        let relative = candidate.relative.as_str();
        info!(relative_path = %relative, "file_discovered");

        let path = match &candidate.state {
            CandidateState::Staged(p) => p,
            CandidateState::Rejected(e) => {
                warn!(error = %e, relative_path = %relative, "file_rejected");
                frames.push(rejected_record(relative, e));
                counts.rejected = counts.rejected.saturating_add(1);
                errors.push(format!("{relative}: {e}"));
                continue;
            }
        };

        if accepted_unique >= config.max_images_per_batch {
            frames.push(FrameRecord {
                frame_id: None,
                relative_path: relative.to_owned(),
                sha256: None,
                intake_status: IntakeStatus::RejectedValidation,
                duplicate_of: None,
                envelope: None,
                error: Some("max_images_per_batch exceeded".into()),
            });
            counts.rejected = counts.rejected.saturating_add(1);
            continue;
        }

        let bytes = match read_file_capped(path, config.max_image_bytes) {
            Ok(b) => b,
            Err(e) => {
                warn!(error = %e, relative_path = %relative, "file_rejected");
                frames.push(rejected_record(relative, &e));
                counts.rejected = counts.rejected.saturating_add(1);
                errors.push(format!("{relative}: {e}"));
                continue;
            }
        };

        // SHA-256 over ORIGINAL bytes (no mutation / re-encode)
        let sha = sha256_hex(&bytes);
        let key = RecoveryKey::new(relative, sha.clone());

        if let Some(prior_id) = recovery.and_then(|ctx| ctx.accepted_frame(&key)) {
            info!(frame_id = %prior_id, relative_path = %relative, "frame_already_accepted");
            accepted_unique = accepted_unique.saturating_add(1);
            counts.skipped_already_accepted = counts.skipped_already_accepted.saturating_add(1);
            seen_sha
                .entry(sha.clone())
                .or_insert_with(|| prior_id.clone());
            frames.push(FrameRecord {
                frame_id: Some(prior_id.clone()),
                relative_path: relative.to_owned(),
                sha256: Some(sha),
                intake_status: IntakeStatus::SkippedAlreadyAccepted,
                duplicate_of: None,
                envelope: None,
                error: None,
            });
            continue;
        }

        let content_type = match validate_image(&bytes, config) {
            Ok(ct) => ct,
            Err(e) => {
                warn!(error = %e, relative_path = %relative, "file_rejected");
                frames.push(rejected_record(relative, &e));
                counts.rejected = counts.rejected.saturating_add(1);
                errors.push(format!("{relative}: {e}"));
                continue;
            }
        };

        // Prove immutability: re-read file and compare
        let again = std::fs::read(path).map_err(|e| TalosError::Permanent(e.to_string()))?;
        if again != bytes {
            return Err(TalosError::Internal(
                "file bytes changed during intake".into(),
            ));
        }
        if sha256_hex(&again) != sha {
            return Err(TalosError::Internal("SHA-256 mismatch on re-hash".into()));
        }

        if let Some(orig) = seen_sha.get(&sha) {
            info!(relative_path = %relative, %sha, "duplicate_detected");
            frames.push(FrameRecord {
                frame_id: None,
                relative_path: relative.to_owned(),
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

        let frame_id = recovery
            .and_then(|ctx| ctx.retry_frame(&key))
            .cloned()
            .unwrap_or_else(new_frame_id);
        seen_sha.insert(sha.clone(), frame_id.clone());

        let mut metadata = talos_types::FrameMetadata::default();
        if let Some(ref mut idx) = meta_index {
            let base_count = basename_counts
                .get(&basename_of(relative))
                .copied()
                .unwrap_or(1);
            let (md, warn) = idx.join(relative, base_count);
            metadata = md;
            if let Some(w) = warn {
                warnings.push(w.clone());
                metadata_warnings = metadata_warnings.saturating_add(1);
            }
        }

        let bytes_ref = format!("file://{}", path.display());
        let mut envelope = IntakeEnvelope::new(
            batch_id.clone(),
            frame_id.clone(),
            SourceRef {
                kind: source_kind.clone(),
                path_or_key: relative.to_owned(),
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
                info!(frame_id = %frame_id, relative_path = %relative, "frame_accepted");
                accepted_unique = accepted_unique.saturating_add(1);
                counts.accepted = counts.accepted.saturating_add(1);
                frames.push(FrameRecord {
                    frame_id: Some(frame_id),
                    relative_path: relative.to_owned(),
                    sha256: Some(sha),
                    intake_status: IntakeStatus::Accepted,
                    duplicate_of: None,
                    envelope: Some(envelope),
                    error: None,
                });
            }
            Err(e) => {
                warn!(error = %e, relative_path = %relative, "sink_failure");
                counts.failed_sink = counts.failed_sink.saturating_add(1);
                frames.push(FrameRecord {
                    frame_id: Some(frame_id),
                    relative_path: relative.to_owned(),
                    sha256: Some(sha),
                    intake_status: IntakeStatus::FailedSink,
                    duplicate_of: None,
                    envelope: Some(envelope),
                    error: Some(e.to_string()),
                });
                errors.push(format!("sink {relative}: {e}"));
            }
        }
    }

    counts.metadata_warnings = metadata_warnings;

    // Prefer Err for empty batches when fail_on_empty_batch (Rejected at API level).
    // A recovery re-run whose frames were all accepted earlier is not empty.
    if counts.accepted == 0 && counts.skipped_already_accepted == 0 && config.fail_on_empty_batch {
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
            path_or_key,
        },
        counts: counts.clone(),
        frames,
        errors,
        warnings,
    };

    if !manifest.reconcile_ok() {
        return Err(TalosError::Internal(format!(
            "manifest reconciliation failed: filesystem_entries_seen={} image_candidates_discovered={} accepted={} rejected={} dup={} sink={} already_accepted={} ignored={}",
            manifest.counts.filesystem_entries_seen,
            manifest.counts.image_candidates_discovered,
            manifest.counts.accepted,
            manifest.counts.rejected,
            manifest.counts.skipped_duplicate,
            manifest.counts.failed_sink,
            manifest.counts.skipped_already_accepted,
            manifest.counts.ignored_unsupported
        )));
    }

    let outcome = batch_outcome(&counts);
    info!(%batch_id, ?outcome, "batch_completed");
    // Staging under staging_root/batch_id is owned by M02 for intake; cleanup is
    // deferred to the orchestrator / M10 (see `plan_cleanup`; nothing deleted here).
    Ok(ImportResult { manifest, outcome })
}

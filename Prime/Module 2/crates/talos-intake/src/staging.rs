//! Staging writes (never overwrite differing evidence) and the pure cleanup planner.

use crate::config::StagingPolicy;
use crate::types::{BatchManifest, IntakeStatus};
use crate::zip_safe::safe_zip_relative_path;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use talos_core::TalosError;
use talos_types::BatchId;

/// Copy `src` to `dest` byte-for-byte. If `dest` already exists (recovery re-run) it must
/// hold identical bytes; differing staged evidence is never overwritten.
pub(crate) fn stage_file(src: &Path, dest: &Path) -> Result<(), TalosError> {
    if dest.exists() {
        let src_bytes = fs::read(src).map_err(|e| TalosError::Permanent(e.to_string()))?;
        return ensure_existing_matches(dest, &src_bytes);
    }
    create_parent(dest)?;
    fs::copy(src, dest).map_err(|e| {
        TalosError::Transient(format!("copy {} → {}: {e}", src.display(), dest.display()))
    })?;
    let src_bytes = fs::read(src).map_err(|e| TalosError::Permanent(e.to_string()))?;
    let dst_bytes = fs::read(dest).map_err(|e| TalosError::Permanent(e.to_string()))?;
    if src_bytes != dst_bytes {
        return Err(TalosError::Internal(
            "staged bytes differ from original — immutability violated".into(),
        ));
    }
    Ok(())
}

/// Persist in-memory bytes (uploads) to `dest` with the same no-overwrite rule.
pub(crate) fn stage_bytes(dest: &Path, bytes: &[u8]) -> Result<(), TalosError> {
    if dest.exists() {
        return ensure_existing_matches(dest, bytes);
    }
    create_parent(dest)?;
    fs::write(dest, bytes)
        .map_err(|e| TalosError::Transient(format!("write {}: {e}", dest.display())))?;
    let dst_bytes = fs::read(dest).map_err(|e| TalosError::Permanent(e.to_string()))?;
    if dst_bytes != bytes {
        return Err(TalosError::Internal(
            "staged bytes differ from upload — immutability violated".into(),
        ));
    }
    Ok(())
}

fn create_parent(dest: &Path) -> Result<(), TalosError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| TalosError::Transient(e.to_string()))?;
    }
    Ok(())
}

fn ensure_existing_matches(dest: &Path, bytes: &[u8]) -> Result<(), TalosError> {
    let existing = fs::read(dest).map_err(|e| TalosError::Permanent(e.to_string()))?;
    if existing != bytes {
        return Err(TalosError::Permanent(format!(
            "staged copy {} already exists with different bytes; refusing to overwrite evidence",
            dest.display()
        )));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupReason {
    RejectedValidation,
    RejectedPermanent,
    SkippedDuplicate,
    BatchDirectory,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupEntry {
    pub path: PathBuf,
    pub reason: CleanupReason,
}

/// Paths that **would** be deleted under a [`StagingPolicy`]. Advisory only: M02 never
/// executes it; deletion belongs to M10 / orchestrator. Entries may name files that were
/// never staged (e.g. uploads rejected before staging), so executors must treat a missing
/// path as a no-op.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupPlan {
    pub batch_id: BatchId,
    pub policy: StagingPolicy,
    pub entries: Vec<CleanupEntry>,
}

impl CleanupPlan {
    pub fn paths(&self) -> Vec<&Path> {
        self.entries.iter().map(|e| e.path.as_path()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Pure planner: no filesystem access, never deletes. Only paths that stay under
/// `staging_root/<batch_id>/` are ever listed (unsafe relative paths are skipped).
///
/// Staged copies of `Accepted`, `FailedSink` (needed for retry) and
/// `SkippedAlreadyAccepted` frames are never proposed under [`StagingPolicy::PlanOnly`].
pub fn plan_cleanup(
    manifest: &BatchManifest,
    staging_root: &Path,
    policy: StagingPolicy,
) -> CleanupPlan {
    let batch_dir = staging_root.join(manifest.batch_id.as_str());
    let batch_dir_safe = safe_zip_relative_path(manifest.batch_id.as_str()).is_ok();
    let mut entries = Vec::new();
    match policy {
        StagingPolicy::Retain => {}
        StagingPolicy::PlanOnly if batch_dir_safe => {
            for frame in &manifest.frames {
                let reason = match frame.intake_status {
                    IntakeStatus::RejectedValidation => CleanupReason::RejectedValidation,
                    IntakeStatus::RejectedPermanent => CleanupReason::RejectedPermanent,
                    IntakeStatus::SkippedDuplicate => CleanupReason::SkippedDuplicate,
                    IntakeStatus::Accepted
                    | IntakeStatus::FailedSink
                    | IntakeStatus::SkippedAlreadyAccepted => continue,
                };
                let Ok(rel) = safe_zip_relative_path(&frame.relative_path) else {
                    continue;
                };
                entries.push(CleanupEntry {
                    path: batch_dir.join(rel),
                    reason,
                });
            }
        }
        StagingPolicy::PlanBatchDir if batch_dir_safe => entries.push(CleanupEntry {
            path: batch_dir,
            reason: CleanupReason::BatchDirectory,
        }),
        StagingPolicy::PlanOnly | StagingPolicy::PlanBatchDir => {}
    }
    CleanupPlan {
        batch_id: manifest.batch_id.clone(),
        policy,
        entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BatchCounts, FrameRecord};
    use talos_types::{SourceKind, SourceRef};

    fn frame(rel: &str, status: IntakeStatus) -> FrameRecord {
        FrameRecord {
            frame_id: None,
            relative_path: rel.into(),
            sha256: None,
            intake_status: status,
            duplicate_of: None,
            envelope: None,
            error: None,
        }
    }

    fn manifest(batch: &str, frames: Vec<FrameRecord>) -> BatchManifest {
        BatchManifest {
            schema_version: "2.0".into(),
            batch_id: BatchId::new(batch),
            source_summary: SourceRef {
                kind: SourceKind::Upload,
                path_or_key: "upload".into(),
            },
            counts: BatchCounts::default(),
            frames,
            errors: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn unsafe_paths_never_planned() {
        let m = manifest(
            "b1",
            vec![
                frame("../escape.jpg", IntakeStatus::RejectedValidation),
                frame("/etc/passwd", IntakeStatus::RejectedValidation),
                frame("ok/bad.jpg", IntakeStatus::RejectedPermanent),
                frame("keep.jpg", IntakeStatus::Accepted),
                frame("retry.jpg", IntakeStatus::FailedSink),
            ],
        );
        let plan = plan_cleanup(&m, Path::new("/stage"), StagingPolicy::PlanOnly);
        assert_eq!(plan.paths(), vec![Path::new("/stage/b1/ok/bad.jpg")]);
    }

    #[test]
    fn unsafe_batch_id_never_planned() {
        let m = manifest(
            "../b1",
            vec![frame("x.jpg", IntakeStatus::SkippedDuplicate)],
        );
        assert!(plan_cleanup(&m, Path::new("/stage"), StagingPolicy::PlanOnly).is_empty());
        assert!(plan_cleanup(&m, Path::new("/stage"), StagingPolicy::PlanBatchDir).is_empty());
    }

    #[test]
    fn retain_plans_nothing() {
        let m = manifest("b1", vec![frame("x.jpg", IntakeStatus::SkippedDuplicate)]);
        assert!(plan_cleanup(&m, Path::new("/stage"), StagingPolicy::Retain).is_empty());
    }
}

//! Staging writes (bounded memory, never overwrite differing evidence) and the pure
//! cleanup planner.

use crate::config::StagingPolicy;
use crate::types::{BatchManifest, IntakeStatus};
use crate::zip_safe::safe_zip_relative_path;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use talos_core::TalosError;
use talos_types::BatchId;

const CHUNK: usize = 64 * 1024;

/// A staged copy and the streaming SHA-256 / length of its (original) bytes.
#[derive(Clone, Debug)]
pub(crate) struct Staged {
    pub path: PathBuf,
    pub sha256: String,
    pub len: u64,
}

pub(crate) fn too_large(len: impl std::fmt::Display, max: u64) -> TalosError {
    TalosError::Validation(format!("file exceeds max_image_bytes ({len} > {max})"))
}

type ReadErr = fn(io::Error) -> TalosError;

fn unreadable(e: io::Error) -> TalosError {
    TalosError::Permanent(format!("unreadable source: {e}"))
}

fn corrupt_entry(e: io::Error) -> TalosError {
    TalosError::Validation(format!("corrupt ZIP entry: {e}"))
}

/// Hash `reader` (optionally teeing into `out`) in fixed-size chunks; fails as soon as
/// more than `max` bytes are seen.
fn pump(
    reader: &mut impl Read,
    mut out: Option<&mut File>,
    max: u64,
    read_err: ReadErr,
) -> Result<(String, u64), TalosError> {
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(read_err(e)),
        };
        total = total.saturating_add(n as u64);
        if total > max {
            return Err(too_large(format!("more than {max}"), max));
        }
        hasher.update(&buf[..n]);
        if let Some(out) = out.as_mut() {
            out.write_all(&buf[..n])
                .map_err(|e| TalosError::Transient(format!("staging write: {e}")))?;
        }
    }
    Ok((format!("{:x}", hasher.finalize()), total))
}

/// Streaming SHA-256 and length of a file, reading at most `max + 1` bytes.
pub(crate) fn hash_file(path: &Path, max: u64) -> Result<(String, u64), TalosError> {
    let mut f = File::open(path).map_err(unreadable)?;
    pump(&mut f, None, max, unreadable)
}

fn partial_path(dest: &Path) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    dest.with_file_name(format!(
        ".{name}.{}-{}.partial",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Write `reader` to a fresh partial file, publish it at `dest` without overwriting
/// (hard link fails if `dest` exists), then re-hash `dest` and compare digest + length.
fn write_new(
    dest: &Path,
    reader: &mut impl Read,
    max: u64,
    read_err: ReadErr,
) -> Result<(String, u64), TalosError> {
    create_parent(dest)?;
    let tmp = partial_path(dest);
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| TalosError::Transient(format!("staging create: {e}")))?;
    let written = pump(reader, Some(&mut out), max, read_err).and_then(|d| {
        out.sync_all()
            .map_err(|e| TalosError::Transient(format!("staging sync: {e}")))?;
        Ok(d)
    });
    drop(out);
    let published = written.and_then(|d| {
        fs::hard_link(&tmp, dest).map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                refuse_overwrite(dest)
            } else {
                TalosError::Transient(format!("staging publish: {e}"))
            }
        })?;
        Ok(d)
    });
    // The partial is this run's own scratch file, never evidence.
    let _ = fs::remove_file(&tmp);
    let (sha, len) = published?;
    let (dest_sha, dest_len) = hash_file(dest, max)?;
    if dest_sha != sha || dest_len != len {
        return Err(TalosError::Internal(
            "staged bytes differ from source — immutability violated".into(),
        ));
    }
    Ok((sha, len))
}

fn refuse_overwrite(dest: &Path) -> TalosError {
    TalosError::Permanent(format!(
        "staged copy {} already exists with different bytes; refusing to overwrite evidence",
        dest.display()
    ))
}

/// Recovery re-run: an existing staged copy must match digest and length exactly.
fn ensure_existing_matches(dest: &Path, sha: &str, len: u64, max: u64) -> Result<(), TalosError> {
    let (dest_sha, dest_len) = hash_file(dest, max).map_err(|_| refuse_overwrite(dest))?;
    if dest_sha != sha || dest_len != len {
        return Err(refuse_overwrite(dest));
    }
    Ok(())
}

/// Copy a folder file to `dest`. Oversize sources are rejected from metadata before any
/// read. Memory is bounded (streamed); the source is hashed while copying and again
/// afterwards, so a source that changed mid-copy is detected.
pub(crate) fn stage_file(src: &Path, dest: &Path, max: u64) -> Result<Staged, TalosError> {
    let meta = fs::metadata(src).map_err(unreadable)?;
    if meta.len() > max {
        return Err(too_large(meta.len(), max));
    }
    let (sha, len) = if dest.exists() {
        let (sha, len) = hash_file(src, max)?;
        ensure_existing_matches(dest, &sha, len, max)?;
        (sha, len)
    } else {
        let mut f = File::open(src).map_err(unreadable)?;
        let copied = write_new(dest, &mut f, max, unreadable)?;
        if hash_file(src, max)? != copied {
            return Err(TalosError::Internal(
                "source bytes changed during staging".into(),
            ));
        }
        copied
    };
    Ok(Staged {
        path: dest.to_path_buf(),
        sha256: sha,
        len,
    })
}

/// Persist in-memory upload bytes to `dest` with the same no-overwrite rule.
pub(crate) fn stage_bytes(dest: &Path, bytes: &[u8], max: u64) -> Result<Staged, TalosError> {
    let (sha, len) = if dest.exists() {
        let (sha, len) = pump(&mut &bytes[..], None, max, unreadable)?;
        ensure_existing_matches(dest, &sha, len, max)?;
        (sha, len)
    } else {
        write_new(dest, &mut &bytes[..], max, unreadable)?
    };
    Ok(Staged {
        path: dest.to_path_buf(),
        sha256: sha,
        len,
    })
}

/// Stream one ZIP entry to `dest`. Read / decompress / CRC failures are `Validation`.
pub(crate) fn stage_reader(
    dest: &Path,
    reader: &mut impl Read,
    max: u64,
) -> Result<Staged, TalosError> {
    let (sha, len) = if dest.exists() {
        let (sha, len) = pump(reader, None, max, corrupt_entry)?;
        ensure_existing_matches(dest, &sha, len, max).map_err(|_| {
            TalosError::Validation(format!(
                "ZIP entry conflicts with existing staged bytes at {}; refusing to overwrite",
                dest.display()
            ))
        })?;
        (sha, len)
    } else {
        write_new(dest, reader, max, corrupt_entry)?
    };
    Ok(Staged {
        path: dest.to_path_buf(),
        sha256: sha,
        len,
    })
}

fn create_parent(dest: &Path) -> Result<(), TalosError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| TalosError::Transient(e.to_string()))?;
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

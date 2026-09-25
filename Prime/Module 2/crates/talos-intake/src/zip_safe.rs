use crate::config::IntakeConfig;
use crate::pipeline::{Candidate, CandidateState, Pending};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use talos_core::TalosError;
use zip::ZipArchive;

/// A vetted archive whose image entries are extracted lazily, one at a time, by the
/// batch loop (so entries past `max_images_per_batch` are never written).
pub(crate) struct ZipPlan {
    pub archive: ZipArchive<File>,
    /// Sorted by relative path. A repeated entry name is a pre-rejected candidate.
    pub candidates: Vec<Candidate>,
    pub ignored_unsupported: u32,
    pub filesystem_entries_seen: u32,
}

/// Validate the whole archive without extracting anything: size, entry count, entry
/// names (zip-slip / absolute), symlinks and the declared uncompressed budget are
/// batch-level `Validation` errors. Non-image entries are counted, never extracted.
pub(crate) fn plan_zip(zip_path: &Path, config: &IntakeConfig) -> Result<ZipPlan, TalosError> {
    let meta = fs::metadata(zip_path).map_err(|e| TalosError::Validation(e.to_string()))?;
    if meta.len() > config.max_zip_bytes {
        return Err(TalosError::Validation(format!(
            "ZIP exceeds max_zip_bytes ({} > {})",
            meta.len(),
            config.max_zip_bytes
        )));
    }

    let file = File::open(zip_path).map_err(|e| TalosError::Validation(e.to_string()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| TalosError::Validation(format!("invalid ZIP: {e}")))?;

    if archive.len() as u32 > config.max_zip_entries {
        return Err(TalosError::Validation(format!(
            "ZIP entry count {} exceeds max_zip_entries {}",
            archive.len(),
            config.max_zip_entries
        )));
    }

    let mut uncompressed_total: u64 = 0;
    let mut candidates = Vec::new();
    let mut used: HashSet<String> = HashSet::new();
    let mut ignored = 0u32;
    let mut seen = 0u32;

    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| TalosError::Validation(format!("ZIP entry: {e}")))?;

        let name = entry.name().to_string();
        if entry.is_symlink() {
            return Err(TalosError::Validation(format!(
                "ZIP symlink rejected: {name}"
            )));
        }
        let rel = safe_zip_relative_path(&name)?;
        if entry.is_dir() {
            continue;
        }
        seen = seen.saturating_add(1);

        uncompressed_total = uncompressed_total.saturating_add(entry.size());
        if uncompressed_total > config.max_uncompressed_zip_bytes {
            return Err(TalosError::Validation(format!(
                "ZIP uncompressed budget exceeded ({} > {})",
                uncompressed_total, config.max_uncompressed_zip_bytes
            )));
        }

        let ext = rel
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !config.extension_allowed(&ext) {
            ignored = ignored.saturating_add(1);
            continue;
        }
        let relative = rel.to_string_lossy().replace('\\', "/");
        let state = if used.insert(relative.clone()) {
            CandidateState::Pending(Pending::ZipEntry(i))
        } else {
            CandidateState::Rejected(TalosError::Validation(format!(
                "duplicate ZIP entry name rejected: {relative}"
            )))
        };
        candidates.push(Candidate { relative, state });
    }

    // Stable: for a repeated name the first (pending) entry stays ahead of its rejection.
    candidates.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(ZipPlan {
        archive,
        candidates,
        ignored_unsupported: ignored,
        filesystem_entries_seen: seen,
    })
}

/// Normalize ZIP entry name to a relative PathBuf under dest; reject traversal/absolute.
pub fn safe_zip_relative_path(name: &str) -> Result<PathBuf, TalosError> {
    let name = name.replace('\\', "/");
    if name.is_empty() {
        return Err(TalosError::Validation("empty ZIP entry name".into()));
    }
    // Absolute (Unix or Windows drive)
    if name.starts_with('/') || name.starts_with('\\') {
        return Err(TalosError::Validation(format!(
            "absolute ZIP path rejected: {name}"
        )));
    }
    if name.chars().nth(1) == Some(':') {
        return Err(TalosError::Validation(format!(
            "absolute ZIP path rejected: {name}"
        )));
    }

    let path = Path::new(&name);
    let mut clean = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Normal(s) => clean.push(s),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(TalosError::Validation(format!(
                    "ZIP-slip (.. ) rejected: {name}"
                )));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(TalosError::Validation(format!(
                    "absolute ZIP path rejected: {name}"
                )));
            }
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(TalosError::Validation(format!(
            "invalid ZIP entry name: {name}"
        )));
    }
    Ok(clean)
}

/// Read file fully into memory with size cap (original bytes; never mutate).
pub fn read_file_capped(path: &Path, max_bytes: u64) -> Result<Vec<u8>, TalosError> {
    let meta = fs::metadata(path).map_err(|e| TalosError::Validation(e.to_string()))?;
    if meta.len() > max_bytes {
        return Err(TalosError::Validation(format!(
            "file exceeds max_image_bytes ({} > {}): {}",
            meta.len(),
            max_bytes,
            path.display()
        )));
    }
    let mut f = File::open(path).map_err(|e| TalosError::Validation(e.to_string()))?;
    let mut buf = Vec::with_capacity(meta.len() as usize);
    f.read_to_end(&mut buf)
        .map_err(|e| TalosError::Permanent(format!("unreadable file {}: {e}", path.display())))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_dir() {
        assert!(safe_zip_relative_path("../etc/passwd").is_err());
        assert!(safe_zip_relative_path("a/../../b").is_err());
    }

    #[test]
    fn rejects_absolute() {
        assert!(safe_zip_relative_path("/etc/passwd").is_err());
        assert!(safe_zip_relative_path("C:/windows").is_err());
    }

    #[test]
    fn accepts_nested() {
        let p = safe_zip_relative_path("cam01/img.jpg").unwrap();
        assert_eq!(p, PathBuf::from("cam01/img.jpg"));
    }
}

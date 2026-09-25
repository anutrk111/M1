//! Library-level upload intake: in-memory files → staging → shared batch pipeline.

use crate::config::IntakeConfig;
use crate::pipeline::{Candidate, CandidateState};
use crate::staging::stage_bytes;
use crate::zip_safe::safe_zip_relative_path;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use talos_core::TalosError;

/// One uploaded file (e.g. a multipart part, once M12 wires HTTP).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadedFile {
    /// Bare file name (`img_0001.jpg`). Separators, `..`, `.`, absolute paths and NUL are
    /// rejected as validation failures.
    pub file_name: String,
    pub bytes: Vec<u8>,
    /// Optional relative directory (`cam01/day1`) under which the file is staged; same
    /// traversal rules as ZIP entry names.
    pub relative_dir: Option<String>,
}

impl UploadedFile {
    pub fn new(file_name: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            file_name: file_name.into(),
            bytes,
            relative_dir: None,
        }
    }

    pub fn with_relative_dir(mut self, dir: impl Into<String>) -> Self {
        self.relative_dir = Some(dir.into());
        self
    }

    /// Display key used in the manifest (not trusted as a filesystem path).
    fn display_path(&self) -> String {
        match &self.relative_dir {
            Some(d) => format!("{}/{}", d.trim_end_matches('/'), self.file_name),
            None => self.file_name.clone(),
        }
    }

    fn safe_relative(&self) -> Result<String, TalosError> {
        let name = self.file_name.as_str();
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains('/')
            || name.contains('\\')
            || name.contains('\0')
            || name.chars().nth(1) == Some(':')
        {
            return Err(TalosError::Validation(format!(
                "unsafe upload file name rejected: {name:?}"
            )));
        }
        let rel = match &self.relative_dir {
            Some(dir) => {
                if dir.contains('\0') {
                    return Err(TalosError::Validation(format!(
                        "unsafe upload relative_dir rejected: {dir:?}"
                    )));
                }
                let dir = safe_zip_relative_path(dir).map_err(|e| {
                    TalosError::Validation(format!("unsafe upload relative_dir: {e}"))
                })?;
                dir.join(name)
            }
            None => PathBuf::from(name),
        };
        Ok(rel.to_string_lossy().replace('\\', "/"))
    }
}

fn extension_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Validate names, persist accepted-extension uploads under `staging`, and return
/// `(candidates, ignored_unsupported, filesystem_entries_seen)` — the same shape as
/// folder discovery. Unsafe, duplicate-path and oversize uploads become pre-rejected
/// candidates (never written to disk).
pub(crate) fn stage_uploads(
    files: Vec<UploadedFile>,
    staging: &Path,
    config: &IntakeConfig,
) -> Result<(Vec<Candidate>, u32, u32), TalosError> {
    let seen = u32::try_from(files.len()).unwrap_or(u32::MAX);
    let mut candidates = Vec::new();
    let mut ignored = 0u32;
    let mut used_paths: HashSet<String> = HashSet::new();

    for file in files {
        let rel = match file.safe_relative() {
            Ok(rel) => rel,
            Err(e) => {
                candidates.push(Candidate {
                    relative: file.display_path(),
                    state: CandidateState::Rejected(e),
                });
                continue;
            }
        };
        if !config.extension_allowed(&extension_of(&file.file_name)) {
            ignored = ignored.saturating_add(1);
            continue;
        }
        if !used_paths.insert(rel.clone()) {
            candidates.push(Candidate {
                relative: rel.clone(),
                state: CandidateState::Rejected(TalosError::Validation(format!(
                    "duplicate upload path rejected: {rel}"
                ))),
            });
            continue;
        }
        let len = file.bytes.len() as u64;
        if len > config.max_image_bytes {
            candidates.push(Candidate {
                relative: rel.clone(),
                state: CandidateState::Rejected(TalosError::Validation(format!(
                    "file exceeds max_image_bytes ({len} > {}): {rel}",
                    config.max_image_bytes
                ))),
            });
            continue;
        }
        let dest = staging.join(&rel);
        stage_bytes(&dest, &file.bytes)?;
        candidates.push(Candidate {
            relative: rel,
            state: CandidateState::Staged(dest),
        });
    }

    candidates.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok((candidates, ignored, seen))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_names() {
        for bad in [
            "../x.jpg", "/abs.jpg", "a/b.jpg", "a\\b.jpg", "..", ".", "", "C:x.jpg", "x\0.jpg",
        ] {
            assert!(
                UploadedFile::new(bad, vec![]).safe_relative().is_err(),
                "{bad:?} should be rejected"
            );
        }
        assert!(UploadedFile::new("x.jpg", vec![])
            .with_relative_dir("../up")
            .safe_relative()
            .is_err());
        assert!(UploadedFile::new("x.jpg", vec![])
            .with_relative_dir("/abs")
            .safe_relative()
            .is_err());
    }

    #[test]
    fn accepts_relative_dir() {
        let rel = UploadedFile::new("x.jpg", vec![])
            .with_relative_dir("cam01/day1")
            .safe_relative()
            .unwrap();
        assert_eq!(rel, "cam01/day1/x.jpg");
    }
}

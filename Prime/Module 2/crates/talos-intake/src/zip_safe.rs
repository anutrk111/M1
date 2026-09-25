use crate::config::IntakeConfig;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use talos_core::TalosError;
use zip::ZipArchive;

/// Extract ZIP safely under `dest_root`. Rejects zip-slip, absolute paths, symlinks.
pub fn extract_zip_safe(
    zip_path: &Path,
    dest_root: &Path,
    config: &IntakeConfig,
) -> Result<(), TalosError> {
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

    fs::create_dir_all(dest_root).map_err(|e| TalosError::Transient(e.to_string()))?;

    let mut uncompressed_total: u64 = 0;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| TalosError::Validation(format!("ZIP entry: {e}")))?;

        let name = entry.name().to_string();
        if entry.is_symlink() {
            return Err(TalosError::Validation(format!(
                "ZIP symlink rejected: {name}"
            )));
        }

        let rel = safe_zip_relative_path(&name)?;
        let out_path = dest_root.join(&rel);

        // Defense in depth: ensure out_path stays under dest_root
        let canon_root =
            fs::canonicalize(dest_root).map_err(|e| TalosError::Transient(e.to_string()))?;
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| TalosError::Transient(e.to_string()))?;
        }
        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| TalosError::Transient(e.to_string()))?;
            continue;
        }

        let size = entry.size();
        uncompressed_total = uncompressed_total.saturating_add(size);
        if uncompressed_total > config.max_uncompressed_zip_bytes {
            return Err(TalosError::Validation(format!(
                "ZIP uncompressed budget exceeded ({} > {})",
                uncompressed_total, config.max_uncompressed_zip_bytes
            )));
        }

        if out_path.exists() {
            // Recovery re-run (or repeated entry name): existing staged bytes must match.
            let mut buf = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
            entry
                .read_to_end(&mut buf)
                .map_err(|e| TalosError::Validation(format!("corrupt ZIP entry {name}: {e}")))?;
            let existing = fs::read(&out_path).map_err(|e| TalosError::Permanent(e.to_string()))?;
            if existing != buf {
                return Err(TalosError::Validation(format!(
                    "ZIP entry {name} conflicts with existing staged bytes; refusing to overwrite"
                )));
            }
        } else {
            let mut outfile =
                File::create(&out_path).map_err(|e| TalosError::Transient(e.to_string()))?;
            copy_entry(&mut entry, &mut outfile, &name)?;
        }

        // Verify path did not escape (after create, check canonicalize of parent)
        if let Ok(canon_out) = fs::canonicalize(&out_path) {
            if !canon_out.starts_with(&canon_root) {
                let _ = fs::remove_file(&out_path);
                return Err(TalosError::Validation(format!(
                    "ZIP-slip rejected after extract: {name}"
                )));
            }
        }
    }

    Ok(())
}

/// Stream an entry to disk. Read/decompress/CRC failures are corrupt input (`Validation`);
/// write failures are staging I/O (`Transient`).
fn copy_entry(entry: &mut impl Read, out: &mut impl Write, name: &str) -> Result<(), TalosError> {
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = entry
            .read(&mut buf)
            .map_err(|e| TalosError::Validation(format!("corrupt ZIP entry {name}: {e}")))?;
        if n == 0 {
            return Ok(());
        }
        out.write_all(&buf[..n])
            .map_err(|e| TalosError::Transient(e.to_string()))?;
    }
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

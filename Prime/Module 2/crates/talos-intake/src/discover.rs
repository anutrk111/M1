use crate::config::IntakeConfig;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct DiscoveredFile {
    pub absolute: PathBuf,
    /// Relative path with `/` separators, no leading `./`.
    pub relative: String,
}

/// Deterministic recursive discovery of allowed image extensions.
/// Returns (image_candidates, ignored_unsupported_count).
pub fn discover_images(
    root: &Path,
    config: &IntakeConfig,
) -> Result<(Vec<DiscoveredFile>, u32), String> {
    if !root.is_dir() {
        return Err(format!("not a directory: {}", root.display()));
    }

    let mut images = Vec::new();
    let mut ignored = 0u32;

    for entry in WalkDir::new(root).follow_links(false).into_iter() {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let rel = rel
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string();

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if config.extension_allowed(&ext) {
            images.push(DiscoveredFile {
                absolute: path.to_path_buf(),
                relative: rel,
            });
        } else {
            ignored = ignored.saturating_add(1);
        }
    }

    images.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok((images, ignored))
}

pub fn normalize_rel_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

pub fn basename_of(rel: &str) -> String {
    rel.rsplit('/').next().unwrap_or(rel).to_string()
}

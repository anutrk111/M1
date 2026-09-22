use crate::discover::{basename_of, normalize_rel_path};
use crate::types::MetadataRecord;
use calamine::{open_workbook_auto, Data, Reader};
use chrono::{DateTime, Utc};
use csv::ReaderBuilder;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use talos_core::TalosError;
use talos_types::{CameraId, FrameMetadata};

#[derive(Default)]
pub struct MetadataIndex {
    by_path: HashMap<String, MetadataRecord>,
    by_basename: HashMap<String, Vec<MetadataRecord>>,
    pub warnings: Vec<String>,
    pub row_errors: u32,
}

impl MetadataIndex {
    pub fn load(path: &Path) -> Result<Self, TalosError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "csv" => Self::load_csv(path),
            "xlsx" | "xls" => Self::load_xlsx(path),
            _ => Err(TalosError::Validation(format!(
                "unsupported metadata sidecar extension: {ext}"
            ))),
        }
    }

    fn load_csv(path: &Path) -> Result<Self, TalosError> {
        let file = File::open(path).map_err(|e| TalosError::Validation(e.to_string()))?;
        let mut rdr = ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(file);
        let headers = rdr
            .headers()
            .map_err(|e| TalosError::Validation(format!("CSV header: {e}")))?
            .clone();
        let map = header_map(&headers);
        let mut idx = MetadataIndex::default();
        for (row_i, rec) in rdr.records().enumerate() {
            match rec {
                Ok(row) => {
                    if let Err(w) = idx.push_row(&map, &row, row_i + 2) {
                        idx.warnings.push(w);
                        idx.row_errors = idx.row_errors.saturating_add(1);
                    }
                }
                Err(e) => {
                    idx.warnings.push(format!("CSV row {}: {e}", row_i + 2));
                    idx.row_errors = idx.row_errors.saturating_add(1);
                }
            }
        }
        Ok(idx)
    }

    fn load_xlsx(path: &Path) -> Result<Self, TalosError> {
        let mut workbook =
            open_workbook_auto(path).map_err(|e| TalosError::Validation(format!("XLSX: {e}")))?;
        let range = workbook
            .worksheet_range_at(0)
            .ok_or_else(|| TalosError::Validation("XLSX has no sheets".into()))?
            .map_err(|e| TalosError::Validation(format!("XLSX sheet: {e}")))?;

        let mut rows = range.rows();
        let header_row = rows
            .next()
            .ok_or_else(|| TalosError::Validation("XLSX empty sheet".into()))?;
        let headers: Vec<String> = header_row
            .iter()
            .map(|c| cell_string(c).to_ascii_lowercase())
            .collect();
        let map = header_map_from_strings(&headers);
        let mut idx = MetadataIndex::default();
        for (i, row) in rows.enumerate() {
            let fields: Vec<String> = row.iter().map(cell_string).collect();
            if fields.iter().all(|s| s.trim().is_empty()) {
                continue;
            }
            if let Err(w) = idx.push_row_strings(&map, &fields, i + 2) {
                idx.warnings.push(w);
                idx.row_errors = idx.row_errors.saturating_add(1);
            }
        }
        Ok(idx)
    }

    fn push_row(
        &mut self,
        map: &HeaderMap,
        row: &csv::StringRecord,
        line: usize,
    ) -> Result<(), String> {
        let get = |key: &str| -> String {
            map.idx(key)
                .and_then(|i| row.get(i).map(|s| s.to_string()))
                .unwrap_or_default()
        };
        self.ingest_fields(
            get("path"),
            get("camera_id"),
            get("captured_at"),
            get("location"),
            line,
        )
    }

    fn push_row_strings(
        &mut self,
        map: &HeaderMap,
        fields: &[String],
        line: usize,
    ) -> Result<(), String> {
        let get = |key: &str| -> String {
            map.idx(key)
                .and_then(|i| fields.get(i).cloned())
                .unwrap_or_default()
        };
        self.ingest_fields(
            get("path"),
            get("camera_id"),
            get("captured_at"),
            get("location"),
            line,
        )
    }

    fn ingest_fields(
        &mut self,
        path_raw: String,
        camera: String,
        captured: String,
        location: String,
        line: usize,
    ) -> Result<(), String> {
        let join_path = normalize_rel_path(path_raw.trim());
        if join_path.is_empty() {
            return Err(format!("metadata line {line}: missing path/file key"));
        }
        // Validate captured_at if present
        if !captured.trim().is_empty() {
            DateTime::parse_from_rfc3339(captured.trim())
                .or_else(|_| DateTime::parse_from_str(captured.trim(), "%Y-%m-%dT%H:%M:%SZ"))
                .map_err(|e| format!("metadata line {line}: bad captured_at: {e}"))?;
        }
        let basename = basename_of(&join_path);
        let rec = MetadataRecord {
            join_path: join_path.clone(),
            basename: basename.clone(),
            camera_id: nonempty(camera),
            captured_at: nonempty(captured),
            location: nonempty(location),
        };
        self.by_path.insert(join_path, rec.clone());
        self.by_basename.entry(basename).or_default().push(rec);
        Ok(())
    }

    /// Deterministic join: relative path primary, then unique basename.
    ///
    /// `batch_basename_count` is how many images in the current batch share this
    /// file's basename. When > 1, basename fallback is treated as ambiguous.
    pub fn join(
        &self,
        relative_path: &str,
        batch_basename_count: u32,
    ) -> (FrameMetadata, Option<String>) {
        let key = normalize_rel_path(relative_path);
        if let Some(rec) = self.by_path.get(&key) {
            return (to_frame_metadata(rec), None);
        }
        let base = basename_of(&key);
        if batch_basename_count > 1 {
            return (
                FrameMetadata::default(),
                Some(format!(
                    "ambiguous basename join for '{base}' ({batch_basename_count} candidates); metadata left null"
                )),
            );
        }
        match self.by_basename.get(&base) {
            Some(list) if list.len() == 1 => (to_frame_metadata(&list[0]), None),
            Some(list) if list.len() > 1 => (
                FrameMetadata::default(),
                Some(format!(
                    "ambiguous basename join for '{base}' ({} candidates); metadata left null",
                    list.len()
                )),
            ),
            _ => (FrameMetadata::default(), None),
        }
    }
}

fn to_frame_metadata(rec: &MetadataRecord) -> FrameMetadata {
    let captured_at = rec.captured_at.as_ref().and_then(|s| {
        DateTime::parse_from_rfc3339(s.trim())
            .ok()
            .map(|d| d.with_timezone(&Utc))
    });
    FrameMetadata {
        camera_id: rec.camera_id.as_ref().map(|c| CameraId::new(c.clone())),
        captured_at,
        location: rec.location.clone(),
    }
}

fn nonempty(s: String) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn cell_string(c: &Data) -> String {
    match c {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("ERR:{e:?}"),
    }
}

struct HeaderMap {
    // canonical key -> column index
    map: HashMap<&'static str, usize>,
}

impl HeaderMap {
    fn idx(&self, key: &str) -> Option<usize> {
        self.map.get(key).copied()
    }
}

fn header_map(headers: &csv::StringRecord) -> HeaderMap {
    let strings: Vec<String> = headers.iter().map(|h| h.to_ascii_lowercase()).collect();
    header_map_from_strings(&strings)
}

fn header_map_from_strings(headers: &[String]) -> HeaderMap {
    let mut map = HashMap::new();
    for (i, h) in headers.iter().enumerate() {
        let h = h.trim();
        let canon = match h {
            "path" | "file" | "relative_path" => Some("path"),
            "camera_id" | "camera" => Some("camera_id"),
            "captured_at" | "timestamp" | "ts" => Some("captured_at"),
            "location" | "loc" => Some("location"),
            _ => None,
        };
        if let Some(c) = canon {
            map.entry(c).or_insert(i);
        }
    }
    HeaderMap { map }
}

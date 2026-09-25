use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use talos_core::TalosError;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntakeConfig {
    pub staging_root: PathBuf,
    pub max_images_per_batch: u32,
    pub max_zip_bytes: u64,
    pub max_image_bytes: u64,
    /// Expanded archive byte budget (zip bomb guard). Default = 2× max_zip_bytes.
    #[serde(default = "default_max_uncompressed_zip_bytes")]
    pub max_uncompressed_zip_bytes: u64,
    #[serde(default = "default_max_zip_entries")]
    pub max_zip_entries: u32,
    pub fail_on_empty_batch: bool,
    pub fail_batch_on_metadata_errors: bool,
    /// When true, any `FailedSink` frame causes the batch to return `Err`.
    /// Default false: batch returns `Ok` with [`BatchOutcome::PartialFailure`](crate::BatchOutcome::PartialFailure).
    #[serde(default = "default_fail_batch_on_sink_errors")]
    pub fail_batch_on_sink_errors: bool,
    pub allowed_extensions: Vec<String>,
    /// Decode-time width cap (pixels).
    #[serde(default = "default_max_width")]
    pub max_width: u32,
    /// Decode-time height cap (pixels).
    #[serde(default = "default_max_height")]
    pub max_height: u32,
    /// Decode-time pixel budget (`width * height`).
    #[serde(default = "default_max_pixel_count")]
    pub max_pixel_count: u64,
    /// What [`plan_cleanup`](crate::plan_cleanup) may propose. M02 never deletes staging.
    #[serde(default)]
    pub staging_policy: StagingPolicy,
}

/// Staging lifecycle policy. Only affects the **plan** returned by
/// [`plan_cleanup`](crate::plan_cleanup); executing deletions belongs to M10 / orchestrator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingPolicy {
    /// Keep everything; the plan is always empty.
    #[default]
    Retain,
    /// Propose staged copies of frames no envelope references
    /// (`RejectedValidation`, `RejectedPermanent`, `SkippedDuplicate`).
    PlanOnly,
    /// Propose the whole `staging_root/<batch_id>/` directory. The executor must first
    /// confirm accepted bytes are durably persisted elsewhere (M10), since accepted
    /// envelopes' `bytes_ref` point into this directory.
    PlanBatchDir,
}

fn default_max_uncompressed_zip_bytes() -> u64 {
    1_073_741_824 // 1 GiB
}

fn default_max_zip_entries() -> u32 {
    50_000
}

fn default_fail_batch_on_sink_errors() -> bool {
    false
}

fn default_max_width() -> u32 {
    8192
}

fn default_max_height() -> u32 {
    8192
}

fn default_max_pixel_count() -> u64 {
    25_000_000
}

impl Default for IntakeConfig {
    fn default() -> Self {
        Self {
            staging_root: PathBuf::from("./data/staging"),
            max_images_per_batch: 10_000,
            max_zip_bytes: 536_870_912,
            max_image_bytes: 26_214_400,
            max_uncompressed_zip_bytes: default_max_uncompressed_zip_bytes(),
            max_zip_entries: default_max_zip_entries(),
            fail_on_empty_batch: true,
            fail_batch_on_metadata_errors: false,
            fail_batch_on_sink_errors: default_fail_batch_on_sink_errors(),
            allowed_extensions: vec!["jpg".into(), "jpeg".into(), "png".into()],
            max_width: default_max_width(),
            max_height: default_max_height(),
            max_pixel_count: default_max_pixel_count(),
            staging_policy: StagingPolicy::default(),
        }
    }
}

impl IntakeConfig {
    pub fn extension_allowed(&self, ext: &str) -> bool {
        let lower = ext.trim_start_matches('.').to_ascii_lowercase();
        self.allowed_extensions
            .iter()
            .any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(&lower))
    }
}

/// Load: defaults → `intake.toml` (if present) → `TALOS__INTAKE__*` env.
pub fn load_intake_config(config_dir: Option<&Path>) -> Result<IntakeConfig, TalosError> {
    let mut figment =
        Figment::new().merge(Serialized::defaults(IntakeConfig::default()).key("intake"));
    if let Some(dir) = config_dir {
        let path = dir.join("intake.toml");
        if path.is_file() {
            figment = figment.merge(Toml::file(&path));
        }
    }
    figment = figment.merge(Env::prefixed("TALOS__").split("__"));
    figment
        .extract_inner::<IntakeConfig>("intake")
        .map_err(|e| TalosError::Config(format!("intake config: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn loads_module_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("intake.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"
[intake]
staging_root = "./tmp-stage"
max_images_per_batch = 42
max_zip_bytes = 1000
max_image_bytes = 500
fail_on_empty_batch = true
fail_batch_on_metadata_errors = false
allowed_extensions = ["jpg", "png"]
"#
        )
        .unwrap();
        let cfg = load_intake_config(Some(dir.path())).unwrap();
        assert_eq!(cfg.max_images_per_batch, 42);
        assert_eq!(cfg.max_image_bytes, 500);
        assert!(!cfg.fail_batch_on_sink_errors);
        assert_eq!(cfg.max_width, 8192);
        assert_eq!(cfg.max_height, 8192);
        assert_eq!(cfg.max_pixel_count, 25_000_000);
        assert!(cfg.extension_allowed("JPG"));
        assert!(!cfg.extension_allowed("gif"));
        assert_eq!(cfg.staging_policy, StagingPolicy::Retain);
    }

    #[test]
    fn parses_staging_policy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("intake.toml"),
            "[intake]\nstaging_policy = \"plan_only\"\n",
        )
        .unwrap();
        let cfg = load_intake_config(Some(dir.path())).unwrap();
        assert_eq!(cfg.staging_policy, StagingPolicy::PlanOnly);
    }
}

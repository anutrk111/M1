//! Layered Talos configuration: defaults → TOML → `TALOS__*` env.

mod error;
mod model;
mod validate;

pub use error::ConfigError;
pub use model::*;

use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use std::path::{Path, PathBuf};

/// Load and validate configuration from a configs directory.
///
/// Load order: compiled defaults → `runtime.toml` / `decision.toml` /
/// `observability.toml` / `pipeline.toml` → `TALOS__*` environment variables.
pub fn load(config_dir: impl AsRef<Path>) -> Result<TalosConfig, ConfigError> {
    load_with(config_dir, true)
}

/// Like [`load`], but optionally skips environment overlays (useful in unit tests).
pub fn load_with(
    config_dir: impl AsRef<Path>,
    merge_env: bool,
) -> Result<TalosConfig, ConfigError> {
    let dir = config_dir.as_ref();
    let mut figment = Figment::from(Serialized::defaults(TalosConfig::defaults()));

    for name in [
        "runtime.toml",
        "decision.toml",
        "observability.toml",
        "pipeline.toml",
    ] {
        let path = dir.join(name);
        if path.is_file() {
            figment = figment.merge(Toml::file(&path));
        }
    }

    if merge_env {
        // TALOS__RUNTIME__SHUTDOWN_GRACE_MS → runtime.shutdown_grace_ms
        figment = figment.merge(Env::prefixed("TALOS__").split("__"));
    }

    let config: TalosConfig = figment
        .extract()
        .map_err(|e| ConfigError::Extract(e.to_string()))?;
    validate::validate(&config)?;
    Ok(config)
}

/// Resolve config directory: `TALOS_CONFIG_DIR`, else `./configs`.
pub fn default_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TALOS_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    PathBuf::from("configs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "talos-config-test-{}-{}-{}",
            std::process::id(),
            nanos,
            n
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn loads_defaults_when_files_missing() {
        let dir = temp_dir();
        let cfg = load_with(&dir, false).unwrap();
        assert!((cfg.decision.auto_approve_min - 0.90).abs() < f32::EPSILON);
        assert!((cfg.decision.secondary_min - 0.80).abs() < f32::EPSILON);
        assert_eq!(cfg.pipeline.default_backend, StageBackend::Fixture);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_inverted_thresholds() {
        let dir = temp_dir();
        fs::write(
            dir.join("decision.toml"),
            "[decision]\nauto_approve_min = 0.70\nsecondary_min = 0.80\n",
        )
        .unwrap();
        let err = load_with(&dir, false).unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_out_of_range_confidence() {
        let dir = temp_dir();
        fs::write(
            dir.join("decision.toml"),
            "[decision]\nauto_approve_min = 1.5\nsecondary_min = 0.80\n",
        )
        .unwrap();
        assert!(load_with(&dir, false).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn loads_repo_configs_without_env() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("configs");
        let cfg = load_with(&root, false).unwrap();
        assert!((cfg.decision.auto_approve_min - 0.90).abs() < f32::EPSILON);
    }
}

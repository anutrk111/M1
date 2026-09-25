use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;
use talos_core::TalosError;
use talos_types::AiOperation;

/// `fixture`: the offline fixture provider may appear in operation routes (CI).
/// `live`: any fixture-kind provider in a route is a boot-time `Config` error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayMode {
    Fixture,
    Live,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AiConfig {
    pub default_mode: GatewayMode,
    pub timeouts_ms: TimeoutsMs,
    pub retry: RetryConfig,
    pub vlm: VlmConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub circuit_breaker: BreakerConfig,
    #[serde(default)]
    pub tie_breaker: TieBreakerConfig,
    /// Upper bound when resolving `file://` `bytes_ref` inputs.
    #[serde(default = "default_max_request_image_bytes")]
    pub max_request_image_bytes: u64,
    pub operations: Operations,
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderConfig>,
}

fn default_max_request_image_bytes() -> u64 {
    26_214_400
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeoutsMs {
    pub detect: u64,
    pub ocr: u64,
    pub hsrp: u64,
    pub vlm: u64,
}

impl TimeoutsMs {
    pub fn for_op(&self, op: AiOperation) -> Duration {
        Duration::from_millis(match op {
            AiOperation::DetectVehiclesPlates => self.detect,
            AiOperation::RunOcr => self.ocr,
            AiOperation::AnalyzeHsrp => self.hsrp,
            AiOperation::VlmAssist => self.vlm,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Additional attempts after the first, `Transient` errors only.
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VlmConfig {
    pub enabled: bool,
    /// Operational guidance only; M08 decides when to invoke VLM. Not enforced here.
    #[serde(default)]
    pub max_fraction: Option<f64>,
}

/// Per-provider token bucket. `requests_per_second <= 0` disables limiting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_second: f64,
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10.0,
            burst: 20,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BreakerConfig {
    /// Consecutive `Transient` failures that open the circuit.
    pub failure_threshold: u32,
    /// How long the circuit stays open before a half-open probe.
    pub open_ms: u64,
    /// Concurrent probes allowed while half-open.
    pub half_open_max_calls: u32,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            open_ms: 30_000,
            half_open_max_calls: 1,
        }
    }
}

/// OCR tie-breaker ("Emperor"): the secondary is consulted on every OCR
/// consensus call; the arbiter only when primary and secondary disagree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TieBreakerConfig {
    pub enabled: bool,
    #[serde(default)]
    pub secondary: Option<String>,
    #[serde(default)]
    pub arbiter: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Operations {
    pub detect: OperationRoute,
    pub ocr: OperationRoute,
    pub hsrp: OperationRoute,
    pub vlm: OperationRoute,
}

impl Operations {
    pub fn route(&self, op: AiOperation) -> &OperationRoute {
        match op {
            AiOperation::DetectVehiclesPlates => &self.detect,
            AiOperation::RunOcr => &self.ocr,
            AiOperation::AnalyzeHsrp => &self.hsrp,
            AiOperation::VlmAssist => &self.vlm,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationRoute {
    pub primary: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

impl OperationRoute {
    pub fn chain(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.primary.as_str()).chain(self.fallbacks.iter().map(String::as_str))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Fixture,
    HttpJson,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub kind: ProviderKind,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub detect_model: Option<String>,
    #[serde(default)]
    pub ocr_model: Option<String>,
    #[serde(default)]
    pub hsrp_model: Option<String>,
    #[serde(default)]
    pub vlm_model: Option<String>,
    /// Env var holding the API key. Default: `TALOS__AI__<NAME>__API_KEY`.
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Overrides the gateway-wide `[ai.rate_limit]`.
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
}

impl ProviderConfig {
    pub fn model_for(&self, op: AiOperation) -> Option<String> {
        match op {
            AiOperation::DetectVehiclesPlates => self.detect_model.clone(),
            AiOperation::RunOcr => self.ocr_model.clone(),
            AiOperation::AnalyzeHsrp => self.hsrp_model.clone(),
            AiOperation::VlmAssist => self.vlm_model.clone(),
        }
        .filter(|m| !m.is_empty())
    }

    pub fn api_key_env_name(&self, provider_name: &str) -> String {
        self.api_key_env.clone().unwrap_or_else(|| {
            format!(
                "TALOS__AI__{}__API_KEY",
                provider_name.to_ascii_uppercase().replace('-', "_")
            )
        })
    }
}

impl AiConfig {
    /// Offline defaults: every operation routed to the fixture provider.
    pub fn fixture_defaults() -> Self {
        let fixture = || OperationRoute {
            primary: "fixture".into(),
            fallbacks: vec![],
        };
        Self {
            default_mode: GatewayMode::Fixture,
            timeouts_ms: TimeoutsMs {
                detect: 10_000,
                ocr: 15_000,
                hsrp: 15_000,
                vlm: 30_000,
            },
            retry: RetryConfig {
                max_retries: 2,
                base_delay_ms: 200,
                max_delay_ms: 5_000,
            },
            vlm: VlmConfig {
                enabled: true,
                max_fraction: Some(0.05),
            },
            rate_limit: RateLimitConfig::default(),
            circuit_breaker: BreakerConfig::default(),
            tie_breaker: TieBreakerConfig::default(),
            max_request_image_bytes: default_max_request_image_bytes(),
            operations: Operations {
                detect: fixture(),
                ocr: fixture(),
                hsrp: fixture(),
                vlm: fixture(),
            },
            providers: BTreeMap::new(),
        }
    }
}

/// Load: defaults → `ai_providers.toml` (if present) → `TALOS__AI__*` env.
/// API keys are read from the environment at gateway build time, never from TOML.
pub fn load_ai_config(config_dir: Option<&Path>) -> Result<AiConfig, TalosError> {
    let mut figment =
        Figment::new().merge(Serialized::defaults(AiConfig::fixture_defaults()).key("ai"));
    if let Some(dir) = config_dir {
        let path = dir.join("ai_providers.toml");
        if path.is_file() {
            figment = figment.merge(Toml::file(&path));
        }
    }
    figment = figment.merge(
        Env::prefixed("TALOS__")
            .filter(|k| !k.as_str().to_ascii_lowercase().ends_with("api_key"))
            .split("__"),
    );
    figment
        .extract_inner::<AiConfig>("ai")
        .map_err(|e| TalosError::Config(format!("ai config: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_committed_module_config() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs");
        let cfg = load_ai_config(Some(&dir)).unwrap();
        assert_eq!(cfg.default_mode, GatewayMode::Fixture);
        assert_eq!(cfg.operations.detect.primary, "fixture");
        assert_eq!(cfg.retry.max_retries, 2);
        assert_eq!(
            cfg.timeouts_ms.for_op(AiOperation::VlmAssist),
            Duration::from_millis(30_000)
        );
        assert!(!cfg.tie_breaker.enabled);
    }

    #[test]
    fn default_api_key_env_name() {
        let p = ProviderConfig {
            enabled: true,
            kind: ProviderKind::HttpJson,
            base_url: None,
            detect_model: None,
            ocr_model: None,
            hsrp_model: None,
            vlm_model: None,
            api_key_env: None,
            rate_limit: None,
        };
        assert_eq!(p.api_key_env_name("open-ai"), "TALOS__AI__OPEN_AI__API_KEY");
    }
}

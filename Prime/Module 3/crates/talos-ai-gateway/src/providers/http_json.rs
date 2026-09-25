//! Generic HTTPS JSON adapter for providers (or provider shims) that speak the
//! Talos normalized wire format.
//!
//! Request: `POST {base_url}/v1/{operation}` with bearer auth and body
//! `{ trace_id, frame_id, batch_id, detection_id, model, content_type,
//!    image_base64, prompt_context }`.
//! Response: `{ "result": <wire>, "usage": { input_tokens, output_tokens, usd_estimate } }`.

use crate::provider::{Provider, ProviderRequest, ProviderResponse};
use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;
use std::fmt;
use std::time::Duration;
use talos_core::TalosError;
use talos_types::AiOperation;

/// API key wrapper that never prints its value.
#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(<redacted>)")
    }
}

pub struct HttpJsonProvider {
    name: String,
    base_url: String,
    api_key: ApiKey,
    models: [Option<String>; 4],
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct Envelope {
    result: serde_json::Value,
    #[serde(default)]
    usage: Usage,
}

#[derive(Deserialize, Default)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    usd_estimate: Option<f64>,
}

fn op_index(op: AiOperation) -> usize {
    match op {
        AiOperation::DetectVehiclesPlates => 0,
        AiOperation::RunOcr => 1,
        AiOperation::AnalyzeHsrp => 2,
        AiOperation::VlmAssist => 3,
    }
}

impl HttpJsonProvider {
    /// `base_url` must be `https://` except loopback hosts (local shims / tests).
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: ApiKey,
        models: [Option<String>; 4],
        request_timeout: Duration,
    ) -> Result<Self, TalosError> {
        let name = name.into();
        let base_url = validate_base_url(&name, &base_url.into())?;
        let client = reqwest::Client::builder()
            .timeout(request_timeout)
            .build()
            .map_err(|e| TalosError::Config(format!("provider {name}: http client: {e}")))?;
        Ok(Self {
            name,
            base_url,
            api_key,
            models,
            client,
        })
    }
}

/// Parse and vet a provider base URL; the bearer key is sent to whatever host this names.
///
/// `https` is required, except plain `http` to an exact loopback host (`localhost`,
/// `127.0.0.0/8`, `::1`) for local shims and tests. Userinfo, query and fragment are
/// rejected. The returned string has no trailing `/`.
pub fn validate_base_url(provider: &str, raw: &str) -> Result<String, TalosError> {
    let bad = |why: &str| TalosError::Config(format!("provider {provider}: base_url {why}"));
    let url = url::Url::parse(raw.trim()).map_err(|_| bad("is not a valid URL"))?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(bad("must not contain userinfo"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(bad("must not contain a query or fragment"));
    }
    let loopback = match url.host() {
        Some(url::Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => return Err(bad("must have a host")),
    };
    match url.scheme() {
        "https" => {}
        "http" if loopback => {}
        _ => return Err(bad("must use https:// (TLS required)")),
    }
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

/// ADR-0007 mapping. Response bodies are not included (they may echo plate text).
pub fn map_status(provider: &str, status: reqwest::StatusCode) -> TalosError {
    let code = status.as_u16();
    let msg = format!("provider {provider} returned HTTP {code}");
    match code {
        408 | 429 | 500 | 502 | 503 | 504 => TalosError::Transient(msg),
        400 | 413 | 415 | 422 => TalosError::Validation(msg),
        401 | 403 => TalosError::Config(msg),
        _ => TalosError::Permanent(msg),
    }
}

#[async_trait]
impl Provider for HttpJsonProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn model_for(&self, op: AiOperation) -> Option<String> {
        self.models[op_index(op)].clone()
    }

    async fn call(&self, req: &ProviderRequest) -> Result<ProviderResponse, TalosError> {
        let url = format!("{}/v1/{}", self.base_url, req.operation.as_str());
        let body = serde_json::json!({
            "trace_id": req.trace_id,
            "frame_id": req.frame_id,
            "batch_id": req.batch_id,
            "detection_id": req.detection_id,
            "model": req.model,
            "content_type": req.content_type,
            "image_base64": base64::engine::general_purpose::STANDARD.encode(req.image.as_slice()),
            "prompt_context": req.prompt_context,
        });
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key.0)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                let kind = if e.is_timeout() {
                    "timeout"
                } else if e.is_connect() {
                    "connect"
                } else {
                    "transport"
                };
                TalosError::Transient(format!("provider {} {kind} error", self.name))
            })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(map_status(&self.name, status));
        }
        let env: Envelope = resp.json().await.map_err(|e| {
            if e.is_timeout() {
                TalosError::Transient(format!("provider {} timeout reading body", self.name))
            } else {
                TalosError::Permanent(format!("provider {} malformed response", self.name))
            }
        })?;
        Ok(ProviderResponse {
            body: env.result,
            input_tokens: env.usage.input_tokens,
            output_tokens: env.usage.output_tokens,
            usd_estimate: env.usage.usd_estimate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping_matches_adr_0007() {
        use reqwest::StatusCode as S;
        let class = |s| map_status("p", s).class_label();
        assert_eq!(class(S::TOO_MANY_REQUESTS), "transient");
        assert_eq!(class(S::BAD_GATEWAY), "transient");
        assert_eq!(class(S::SERVICE_UNAVAILABLE), "transient");
        assert_eq!(class(S::GATEWAY_TIMEOUT), "transient");
        assert_eq!(class(S::BAD_REQUEST), "validation");
        assert_eq!(class(S::UNAUTHORIZED), "config");
        assert_eq!(class(S::FORBIDDEN), "config");
        assert_eq!(class(S::NOT_FOUND), "permanent");
    }

    #[test]
    fn api_key_debug_is_redacted() {
        let k = ApiKey::new("sk-secret");
        assert!(!format!("{k:?}").contains("sk-secret"));
    }

    #[test]
    fn plain_http_rejected_except_loopback() {
        let mk = |u: &str| {
            HttpJsonProvider::new(
                "p",
                u,
                ApiKey::new("k"),
                Default::default(),
                Duration::from_secs(1),
            )
        };
        assert!(mk("http://api.example.com").is_err());
        assert!(mk("https://api.example.com").is_ok());
        assert!(mk("http://127.0.0.1:9999").is_ok());
    }

    #[test]
    fn exact_loopback_http_accepted() {
        for u in [
            "http://localhost",
            "http://LOCALHOST:8080/",
            "http://127.0.0.1:9999",
            "http://127.1.2.3",
            "http://[::1]:7000/shim",
            "https://api.example.com/base/",
        ] {
            assert!(validate_base_url("p", u).is_ok(), "{u} should be accepted");
        }
        assert_eq!(
            validate_base_url("p", "https://api.example.com/base/").unwrap(),
            "https://api.example.com/base"
        );
    }

    #[test]
    fn loopback_lookalikes_and_malformed_rejected_as_config() {
        for u in [
            "http://localhost.evil.example",
            "http://localhost.evil.example:80/v1",
            "http://127.0.0.1.evil.example",
            "http://example.com?host=localhost",
            "http://example.com/#localhost",
            "http://localhost@evil.example",
            "http://127.0.0.1@evil.example",
            "https://user:pw@api.example.com",
            "http://[::2]",
            "http://10.0.0.1",
            "http://localhost./",
            "ftp://localhost",
            "file:///etc/passwd",
            "localhost:8080",
            "not a url",
            "",
        ] {
            let err = validate_base_url("p", u).err();
            assert!(
                matches!(err, Some(TalosError::Config(_))),
                "{u:?} should be a Config error, got {err:?}"
            );
        }
    }

    #[test]
    fn config_errors_do_not_echo_the_url() {
        let err = validate_base_url("p", "http://user:sk-secret@evil.example")
            .unwrap_err()
            .to_string();
        assert!(!err.contains("sk-secret"));
    }
}

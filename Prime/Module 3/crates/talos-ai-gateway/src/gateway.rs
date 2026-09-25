use crate::config::{AiConfig, GatewayMode, ProviderKind};
use crate::cost::{CostRecorder, GatewayMetrics, MetricsSnapshot, NoopCostRecorder};
use crate::normalize;
use crate::policy::{Backoff, BreakerState, CircuitBreaker, TokenBucket};
use crate::provider::{Provider, ProviderRequest, ProviderResponse};
use crate::providers::{ApiKey, FixtureProvider, HttpJsonProvider};
use async_trait::async_trait;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use talos_core::{
    OcrConsensus, TalosError, VisionGateway, VisionRequest, VlmAssistResult, VlmRequest,
};
use talos_types::{
    AiOperation, CostEvent, CostStatus, Detections, HsrpEvidence, OcrHypothesis, ProviderRef,
};
use tokio::time::Instant;

/// Point-in-time provider health for ops dashboards and M12 admin views.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProviderHealth {
    pub name: String,
    pub breaker: BreakerState,
    pub consecutive_failures: u32,
    pub ok_total: u64,
    pub error_total: u64,
    pub last_error_class: Option<String>,
}

#[derive(Default)]
struct Stats {
    ok_total: u64,
    error_total: u64,
    last_error_class: Option<String>,
}

struct Slot {
    provider: Arc<dyn Provider>,
    breaker: CircuitBreaker,
    bucket: TokenBucket,
    stats: Mutex<Stats>,
}

pub struct GatewayBuilder {
    cfg: AiConfig,
    extra: Vec<Arc<dyn Provider>>,
    cost: Arc<dyn CostRecorder>,
}

impl GatewayBuilder {
    /// Register (or replace by name) a provider adapter.
    pub fn with_provider(mut self, provider: Arc<dyn Provider>) -> Self {
        self.extra.push(provider);
        self
    }

    pub fn with_cost_recorder(mut self, cost: Arc<dyn CostRecorder>) -> Self {
        self.cost = cost;
        self
    }

    /// Fails closed: unknown providers in routes, fixture providers in `live`
    /// mode, or an enabled HTTP provider without its API key → `Config`.
    pub fn build(self) -> Result<AiGateway, TalosError> {
        let cfg = self.cfg;
        let mut providers: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();
        let request_timeout = [
            cfg.timeouts_ms.detect,
            cfg.timeouts_ms.ocr,
            cfg.timeouts_ms.hsrp,
            cfg.timeouts_ms.vlm,
        ]
        .into_iter()
        .max()
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_secs(30));

        for (name, pc) in cfg.providers.iter().filter(|(_, pc)| pc.enabled) {
            let provider: Arc<dyn Provider> = match pc.kind {
                ProviderKind::Fixture => Arc::new(FixtureProvider::new(name.clone())),
                ProviderKind::HttpJson => {
                    let base_url =
                        pc.base_url
                            .clone()
                            .filter(|u| !u.is_empty())
                            .ok_or_else(|| {
                                TalosError::Config(format!("provider {name}: base_url required"))
                            })?;
                    let env_name = pc.api_key_env_name(name);
                    let key = std::env::var(&env_name)
                        .ok()
                        .filter(|k| !k.trim().is_empty())
                        .ok_or_else(|| {
                            TalosError::Config(format!(
                                "provider {name} is enabled but {env_name} is not set"
                            ))
                        })?;
                    Arc::new(HttpJsonProvider::new(
                        name.clone(),
                        base_url,
                        ApiKey::new(key),
                        [
                            pc.model_for(AiOperation::DetectVehiclesPlates),
                            pc.model_for(AiOperation::RunOcr),
                            pc.model_for(AiOperation::AnalyzeHsrp),
                            pc.model_for(AiOperation::VlmAssist),
                        ],
                        request_timeout,
                    )?)
                }
            };
            providers.insert(name.clone(), provider);
        }
        if cfg.default_mode == GatewayMode::Fixture && !providers.contains_key("fixture") {
            providers.insert("fixture".into(), Arc::new(FixtureProvider::new("fixture")));
        }
        for p in self.extra {
            providers.insert(p.name().to_owned(), p);
        }

        let check = |name: &str, role: &str| -> Result<(), TalosError> {
            let p = providers.get(name).ok_or_else(|| {
                TalosError::Config(format!(
                    "{role} references unknown or disabled provider {name}"
                ))
            })?;
            if cfg.default_mode == GatewayMode::Live && p.is_fixture() {
                return Err(TalosError::Config(format!(
                    "{role} routes to fixture provider {name} in live mode"
                )));
            }
            Ok(())
        };
        for op in [
            AiOperation::DetectVehiclesPlates,
            AiOperation::RunOcr,
            AiOperation::AnalyzeHsrp,
            AiOperation::VlmAssist,
        ] {
            if op == AiOperation::VlmAssist && !cfg.vlm.enabled {
                continue;
            }
            for name in cfg.operations.route(op).chain() {
                check(name, op.as_str())?;
            }
        }
        if cfg.tie_breaker.enabled {
            for (role, name) in [
                ("tie_breaker.secondary", &cfg.tie_breaker.secondary),
                ("tie_breaker.arbiter", &cfg.tie_breaker.arbiter),
            ] {
                let name = name.as_deref().ok_or_else(|| {
                    TalosError::Config(format!("{role} must be set when enabled"))
                })?;
                check(name, role)?;
            }
        }

        let slots = providers
            .into_iter()
            .map(|(name, provider)| {
                let rl = cfg
                    .providers
                    .get(&name)
                    .and_then(|pc| pc.rate_limit.clone())
                    .unwrap_or_else(|| cfg.rate_limit.clone());
                let slot = Slot {
                    provider,
                    breaker: CircuitBreaker::new(cfg.circuit_breaker.clone()),
                    bucket: TokenBucket::new(&rl),
                    stats: Mutex::new(Stats::default()),
                };
                (name, slot)
            })
            .collect();

        Ok(AiGateway {
            backoff: Backoff::new(&cfg.retry),
            cfg,
            slots,
            cost: self.cost,
            metrics: GatewayMetrics::default(),
        })
    }
}

/// M03 gateway: routes each operation through its provider chain with
/// timeout, rate limit, circuit breaker, bounded retry and ordered failover.
pub struct AiGateway {
    cfg: AiConfig,
    slots: BTreeMap<String, Slot>,
    cost: Arc<dyn CostRecorder>,
    metrics: GatewayMetrics,
    backoff: Backoff,
}

enum Attempt {
    Called(Result<ProviderResponse, TalosError>),
    /// Rejected locally before reaching the provider (no cost).
    Local(TalosError),
}

impl AiGateway {
    pub fn builder(cfg: AiConfig) -> GatewayBuilder {
        GatewayBuilder {
            cfg,
            extra: Vec::new(),
            cost: Arc::new(NoopCostRecorder),
        }
    }

    pub fn from_config(cfg: AiConfig, cost: Arc<dyn CostRecorder>) -> Result<Self, TalosError> {
        Self::builder(cfg).with_cost_recorder(cost).build()
    }

    pub fn config(&self) -> &AiConfig {
        &self.cfg
    }

    pub fn health(&self) -> Vec<ProviderHealth> {
        self.slots
            .iter()
            .map(|(name, s)| {
                let st = s.stats.lock().unwrap_or_else(|p| p.into_inner());
                ProviderHealth {
                    name: name.clone(),
                    breaker: s.breaker.state(),
                    consecutive_failures: s.breaker.consecutive_failures(),
                    ok_total: st.ok_total,
                    error_total: st.error_total,
                    last_error_class: st.last_error_class.clone(),
                }
            })
            .collect()
    }

    pub fn metrics(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }

    async fn resolve_image(&self, req: &VisionRequest) -> Result<Arc<Vec<u8>>, TalosError> {
        if let Some(bytes) = &req.image_bytes {
            if bytes.len() as u64 > self.cfg.max_request_image_bytes {
                return Err(TalosError::Validation(
                    "image exceeds max_request_image_bytes".into(),
                ));
            }
            return Ok(Arc::new(bytes.clone()));
        }
        let r = req.bytes_ref.as_deref().unwrap_or_default();
        let Some(path) = r.strip_prefix("file://") else {
            return Err(TalosError::NotImplemented(format!(
                "bytes_ref scheme for {r:?} (object store resolution is M10)"
            )));
        };
        let meta = tokio::fs::metadata(path)
            .await
            .map_err(|_| TalosError::Permanent(format!("bytes_ref unreadable: {r}")))?;
        if meta.len() > self.cfg.max_request_image_bytes {
            return Err(TalosError::Validation(
                "image exceeds max_request_image_bytes".into(),
            ));
        }
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|_| TalosError::Permanent(format!("bytes_ref unreadable: {r}")))?;
        Ok(Arc::new(bytes))
    }

    async fn attempt(&self, slot: &Slot, preq: &ProviderRequest, timeout: Duration) -> Attempt {
        if !slot.breaker.try_acquire() {
            return Attempt::Local(TalosError::Transient(format!(
                "provider {} circuit open",
                slot.provider.name()
            )));
        }
        let started = Instant::now();
        if tokio::time::timeout(timeout, slot.bucket.acquire())
            .await
            .is_err()
        {
            slot.breaker.on_neutral();
            return Attempt::Local(TalosError::Transient(format!(
                "provider {} rate limit wait exceeded timeout",
                slot.provider.name()
            )));
        }
        let remaining = timeout.saturating_sub(started.elapsed());
        let res = match tokio::time::timeout(remaining, slot.provider.call(preq)).await {
            Ok(r) => r,
            Err(_) => Err(TalosError::Transient(format!(
                "provider {} timed out",
                slot.provider.name()
            ))),
        };
        match &res {
            Ok(_) => slot.breaker.on_success(),
            Err(e) if e.is_retryable() => slot.breaker.on_failure(),
            Err(_) => slot.breaker.on_neutral(),
        }
        Attempt::Called(res)
    }

    fn account(
        &self,
        slot: &Slot,
        preq: &ProviderRequest,
        res: &Result<ProviderResponse, TalosError>,
        latency: Duration,
    ) {
        let name = slot.provider.name();
        let op = preq.operation.as_str();
        let status = match res {
            Ok(_) => "ok",
            Err(e) => e.class_label(),
        };
        self.metrics
            .request(name, op, status, latency.as_secs_f64());
        {
            let mut st = slot.stats.lock().unwrap_or_else(|p| p.into_inner());
            match res {
                Ok(_) => st.ok_total += 1,
                Err(e) => {
                    st.error_total += 1;
                    st.last_error_class = Some(e.class_label().to_owned());
                }
            }
        }
        // Credential / wiring failures never reach a billable model call.
        let billable = !matches!(
            res,
            Err(TalosError::Config(_) | TalosError::NotImplemented(_) | TalosError::Internal(_))
        );
        if !billable {
            return;
        }
        let (tokens_in, tokens_out, usd) = match res {
            Ok(r) => (r.input_tokens, r.output_tokens, r.usd_estimate),
            Err(_) => (None, None, None),
        };
        if let Some(usd) = usd {
            self.metrics
                .cost(name, preq.model.as_deref().unwrap_or("unknown"), usd);
        }
        self.cost.record(CostEvent {
            trace_id: preq.trace_id.clone(),
            frame_id: preq.frame_id.clone(),
            batch_id: preq.batch_id.clone(),
            provider: name.to_owned(),
            model: preq.model.clone(),
            operation: preq.operation,
            status: if res.is_ok() {
                CostStatus::Ok
            } else {
                CostStatus::Error
            },
            latency_ms: latency.as_millis() as u64,
            input_tokens: tokens_in,
            output_tokens: tokens_out,
            image_units: 1,
            usd_estimate: usd,
            error_class: res.as_ref().err().map(|e| e.class_label().to_owned()),
        });
    }

    /// Retries `Transient` up to `max_retries`, then the caller fails over.
    async fn try_provider(
        &self,
        slot: &Slot,
        preq: &ProviderRequest,
        timeout: Duration,
    ) -> Result<ProviderResponse, TalosError> {
        let mut last = None;
        for attempt in 0..=self.cfg.retry.max_retries {
            let started = Instant::now();
            match self.attempt(slot, preq, timeout).await {
                Attempt::Local(e) => return Err(e),
                Attempt::Called(res) => {
                    self.account(slot, preq, &res, started.elapsed());
                    match res {
                        Ok(r) => return Ok(r),
                        Err(e) if e.is_retryable() => {
                            tracing::warn!(
                                provider = slot.provider.name(),
                                operation = preq.operation.as_str(),
                                attempt,
                                "transient provider error"
                            );
                            last = Some(e);
                            if attempt < self.cfg.retry.max_retries {
                                tokio::time::sleep(self.backoff.delay(attempt)).await;
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        Err(last.unwrap_or_else(|| TalosError::Internal("retry loop without attempt".into())))
    }

    async fn execute<'a>(
        &self,
        op: AiOperation,
        req: &VisionRequest,
        prompt_context: Option<String>,
        chain: impl IntoIterator<Item = &'a str>,
    ) -> Result<(ProviderResponse, ProviderRef), TalosError> {
        req.validate()?;
        let image = self.resolve_image(req).await?;
        let timeout = self.cfg.timeouts_ms.for_op(op);
        let mut prev: Option<&str> = None;
        let mut last = None;
        for name in chain {
            let slot = self
                .slots
                .get(name)
                .ok_or_else(|| TalosError::Config(format!("unknown provider {name}")))?;
            if let Some(from) = prev {
                self.metrics.fallback(op.as_str(), from, name);
                tracing::warn!(
                    operation = op.as_str(),
                    from,
                    to = name,
                    "provider failover"
                );
            }
            let model = slot.provider.model_for(op);
            let preq = ProviderRequest {
                operation: op,
                trace_id: req.trace_id.clone(),
                frame_id: req.frame_id.clone(),
                batch_id: req.batch_id.clone(),
                detection_id: req.detection_id.clone(),
                content_type: req.content_type.clone(),
                image: image.clone(),
                prompt_context: prompt_context.clone(),
                model: model.clone(),
            };
            match self.try_provider(slot, &preq, timeout).await {
                Ok(resp) => {
                    return Ok((
                        resp,
                        ProviderRef {
                            name: name.to_owned(),
                            model,
                        },
                    ))
                }
                Err(e) if e.is_retryable() => last = Some(e),
                Err(e) => return Err(e),
            }
            prev = Some(name);
        }
        Err(last.unwrap_or_else(|| TalosError::Config("empty provider route".into())))
    }

    async fn routed(
        &self,
        op: AiOperation,
        req: &VisionRequest,
        prompt: Option<String>,
    ) -> Result<(ProviderResponse, ProviderRef), TalosError> {
        let route = self.cfg.operations.route(op).clone();
        self.execute(op, req, prompt, route.chain()).await
    }

    async fn ocr_via(
        &self,
        req: &VisionRequest,
        provider: &str,
    ) -> Result<Vec<OcrHypothesis>, TalosError> {
        let det = req.require_detection_id()?.clone();
        let (resp, pref) = self
            .execute(AiOperation::RunOcr, req, None, [provider])
            .await?;
        normalize::ocr(&pref, &det, resp.body)
    }
}

fn top_text(h: &[OcrHypothesis]) -> Option<String> {
    h.iter()
        .max_by(|a, b| a.confidence.total_cmp(&b.confidence))
        .map(|t| {
            t.text
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_uppercase())
                .collect()
        })
}

#[async_trait]
impl VisionGateway for AiGateway {
    #[tracing::instrument(name = "ai.detect", skip_all, fields(trace_id = %req.trace_id, frame_id = %req.frame_id))]
    async fn detect_vehicles_plates(&self, req: VisionRequest) -> Result<Detections, TalosError> {
        let (resp, pref) = self
            .routed(AiOperation::DetectVehiclesPlates, &req, None)
            .await?;
        normalize::detections(&pref, resp.body)
    }

    #[tracing::instrument(name = "ai.ocr", skip_all, fields(trace_id = %req.trace_id, frame_id = %req.frame_id))]
    async fn run_ocr(&self, req: VisionRequest) -> Result<Vec<OcrHypothesis>, TalosError> {
        let det = req.require_detection_id()?.clone();
        let (resp, pref) = self.routed(AiOperation::RunOcr, &req, None).await?;
        normalize::ocr(&pref, &det, resp.body)
    }

    #[tracing::instrument(name = "ai.hsrp", skip_all, fields(trace_id = %req.trace_id, frame_id = %req.frame_id))]
    async fn analyze_hsrp(&self, req: VisionRequest) -> Result<HsrpEvidence, TalosError> {
        let det = req.require_detection_id()?.clone();
        let (resp, pref) = self.routed(AiOperation::AnalyzeHsrp, &req, None).await?;
        normalize::hsrp(&pref, &det, resp.body)
    }

    #[tracing::instrument(name = "ai.vlm", skip_all, fields(trace_id = %req.vision.trace_id, frame_id = %req.vision.frame_id))]
    async fn vlm_assist(&self, req: VlmRequest) -> Result<VlmAssistResult, TalosError> {
        if !self.cfg.vlm.enabled {
            return Err(TalosError::NotImplemented(
                "vlm_assist disabled by [ai.vlm].enabled".into(),
            ));
        }
        let (resp, pref) = self
            .routed(
                AiOperation::VlmAssist,
                &req.vision,
                Some(req.prompt_context),
            )
            .await?;
        let (evidence, hints) = normalize::vlm(&pref, resp.body)?;
        Ok(VlmAssistResult { evidence, hints })
    }

    /// Secondary / arbiter failures leave their slot `None` (opinion absent),
    /// never a fabricated agreement.
    async fn run_ocr_consensus(&self, req: VisionRequest) -> Result<OcrConsensus, TalosError> {
        let primary = self.run_ocr(req.clone()).await?;
        let tb = &self.cfg.tie_breaker;
        let (Some(secondary_name), Some(arbiter_name), true) =
            (tb.secondary.as_deref(), tb.arbiter.as_deref(), tb.enabled)
        else {
            return Ok(OcrConsensus {
                primary,
                ..OcrConsensus::default()
            });
        };
        let secondary = match self.ocr_via(&req, secondary_name).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    provider = secondary_name,
                    class = e.class_label(),
                    "secondary OCR unavailable"
                );
                return Ok(OcrConsensus {
                    primary,
                    ..OcrConsensus::default()
                });
            }
        };
        let agreed = top_text(&primary).is_some() && top_text(&primary) == top_text(&secondary);
        let arbiter = if agreed {
            None
        } else {
            match self.ocr_via(&req, arbiter_name).await {
                Ok(a) => Some(a),
                Err(e) => {
                    tracing::warn!(
                        provider = arbiter_name,
                        class = e.class_label(),
                        "arbiter OCR unavailable"
                    );
                    None
                }
            }
        };
        Ok(OcrConsensus {
            primary,
            secondary: Some(secondary),
            arbiter,
            agreed: Some(agreed),
        })
    }
}

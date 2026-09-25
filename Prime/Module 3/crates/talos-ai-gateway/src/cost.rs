//! Cost accounting and in-process metrics. Long-term persistence is M10.

use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Mutex;
use talos_types::CostEvent;

/// Metric names (M03 PDR section 9.2).
pub mod metric {
    pub const REQUESTS_TOTAL: &str = "talos_ai_requests_total";
    pub const LATENCY_SECONDS: &str = "talos_ai_latency_seconds";
    pub const COST_USD_TOTAL: &str = "talos_ai_cost_usd_total";
    pub const FALLBACKS_TOTAL: &str = "talos_ai_fallbacks_total";
}

pub trait CostRecorder: Send + Sync {
    fn record(&self, event: CostEvent);
}

#[derive(Default)]
pub struct InMemoryCostRecorder {
    events: Mutex<Vec<CostEvent>>,
}

impl InMemoryCostRecorder {
    pub fn events(&self) -> Vec<CostEvent> {
        self.events
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
}

impl CostRecorder for InMemoryCostRecorder {
    fn record(&self, event: CostEvent) {
        self.events
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(event);
    }
}

/// Emits nothing; for callers that do not account costs.
pub struct NoopCostRecorder;

impl CostRecorder for NoopCostRecorder {
    fn record(&self, _event: CostEvent) {}
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct LatencyStat {
    pub count: u64,
    pub sum_seconds: f64,
}

/// Snapshot of in-process series. Keys are `name{label=value,...}`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct MetricsSnapshot {
    pub counters: BTreeMap<String, u64>,
    pub cost_usd: BTreeMap<String, f64>,
    pub latency: BTreeMap<String, LatencyStat>,
}

impl MetricsSnapshot {
    pub fn counter(&self, key: &str) -> u64 {
        self.counters.get(key).copied().unwrap_or(0)
    }
}

#[derive(Default)]
pub struct GatewayMetrics {
    inner: Mutex<MetricsSnapshot>,
}

impl GatewayMetrics {
    fn with<R>(&self, f: impl FnOnce(&mut MetricsSnapshot) -> R) -> R {
        f(&mut self.inner.lock().unwrap_or_else(|p| p.into_inner()))
    }

    pub fn request(&self, provider: &str, operation: &str, status: &str, seconds: f64) {
        self.with(|m| {
            *m.counters
                .entry(format!(
                    "{}{{provider={provider},operation={operation},status={status}}}",
                    metric::REQUESTS_TOTAL
                ))
                .or_default() += 1;
            let l = m
                .latency
                .entry(format!(
                    "{}{{provider={provider},operation={operation}}}",
                    metric::LATENCY_SECONDS
                ))
                .or_default();
            l.count += 1;
            l.sum_seconds += seconds;
        });
    }

    pub fn cost(&self, provider: &str, model: &str, usd: f64) {
        self.with(|m| {
            *m.cost_usd
                .entry(format!(
                    "{}{{provider={provider},model={model}}}",
                    metric::COST_USD_TOTAL
                ))
                .or_default() += usd;
        });
    }

    pub fn fallback(&self, operation: &str, from: &str, to: &str) {
        self.with(|m| {
            *m.counters
                .entry(format!(
                    "{}{{operation={operation},from_provider={from},to_provider={to}}}",
                    metric::FALLBACKS_TOTAL
                ))
                .or_default() += 1;
        });
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        self.with(|m| m.clone())
    }
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TalosConfig {
    pub runtime: RuntimeConfig,
    pub decision: DecisionConfig,
    pub observability: ObservabilityConfig,
    pub pipeline: PipelineConfig,
}

impl TalosConfig {
    pub fn defaults() -> Self {
        Self {
            runtime: RuntimeConfig::defaults(),
            decision: DecisionConfig::defaults(),
            observability: ObservabilityConfig::defaults(),
            pipeline: PipelineConfig::defaults(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub worker_threads: usize,
    pub shutdown_grace_ms: u64,
    pub max_in_flight_frames: usize,
}

impl RuntimeConfig {
    pub fn defaults() -> Self {
        Self {
            worker_threads: 4,
            shutdown_grace_ms: 15_000,
            max_in_flight_frames: 64,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionConfig {
    pub auto_approve_min: f32,
    pub secondary_min: f32,
    pub hard_fail_on_grammar: bool,
    pub hard_fail_on_no_plate: bool,
}

impl DecisionConfig {
    pub fn defaults() -> Self {
        Self {
            auto_approve_min: 0.90,
            secondary_min: 0.80,
            hard_fail_on_grammar: true,
            hard_fail_on_no_plate: true,
        }
    }

    /// Map fused confidence (+ hard-fail) to a decision outcome.
    pub fn outcome_for(
        &self,
        fused_confidence: f32,
        hard_fail: bool,
    ) -> talos_types::DecisionOutcome {
        use talos_types::DecisionOutcome::*;
        if hard_fail || fused_confidence < self.secondary_min {
            return ReviewRequired;
        }
        if fused_confidence >= self.auto_approve_min {
            AutoApproved
        } else {
            SecondaryVerification
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub log_level: String,
    pub json_logs: bool,
    pub trace_sample_rate: f64,
    pub allow_plate_debug: bool,
}

impl ObservabilityConfig {
    pub fn defaults() -> Self {
        Self {
            log_level: "info".into(),
            json_logs: true,
            trace_sample_rate: 1.0,
            allow_plate_debug: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageBackend {
    Fixture,
    AiApi,
    LocalGpuExt,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub default_backend: StageBackend,
    #[serde(default)]
    pub stages: StageBackendOverrides,
}

impl PipelineConfig {
    pub fn defaults() -> Self {
        Self {
            default_backend: StageBackend::Fixture,
            stages: StageBackendOverrides::default(),
        }
    }

    pub fn backend_for(&self, stage_index: u8) -> StageBackend {
        let override_backend = match stage_index {
            0 => self.stages.s0,
            1 => self.stages.s1,
            2 => self.stages.s2,
            3 => self.stages.s3,
            4 => self.stages.s4,
            5 => self.stages.s5,
            6 => self.stages.s6,
            7 => self.stages.s7,
            _ => None,
        };
        override_backend.unwrap_or(self.default_backend)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct StageBackendOverrides {
    pub s0: Option<StageBackend>,
    pub s1: Option<StageBackend>,
    pub s2: Option<StageBackend>,
    pub s3: Option<StageBackend>,
    pub s4: Option<StageBackend>,
    pub s5: Option<StageBackend>,
    pub s6: Option<StageBackend>,
    pub s7: Option<StageBackend>,
}

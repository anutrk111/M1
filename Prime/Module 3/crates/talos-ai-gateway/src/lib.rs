//! M03 AI Provider Gateway — sole egress for remote vision / VLM APIs.
//!
//! Implements the M01 port [`talos_core::VisionGateway`]. See
//! `Prime/Module 3/docs/pdr/M03-ai-provider-gateway.md`.

pub mod config;
pub mod cost;
pub mod gateway;
pub mod normalize;
pub mod policy;
pub mod provider;
pub mod providers;

pub use config::{load_ai_config, AiConfig, GatewayMode};
pub use cost::{CostRecorder, InMemoryCostRecorder, MetricsSnapshot, NoopCostRecorder};
pub use gateway::{AiGateway, GatewayBuilder, ProviderHealth};
pub use policy::BreakerState;
pub use provider::{Provider, ProviderRequest, ProviderResponse};
pub use talos_core::{OcrConsensus, VisionGateway, VisionRequest, VlmAssistResult, VlmRequest};

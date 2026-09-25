use async_trait::async_trait;
use std::sync::Arc;
use talos_core::TalosError;
use talos_types::{AiOperation, BatchId, DetectionId, FrameId, TraceId};

/// What a provider adapter receives. Image bytes are already resolved.
#[derive(Clone, Debug)]
pub struct ProviderRequest {
    pub operation: AiOperation,
    pub trace_id: TraceId,
    pub frame_id: FrameId,
    pub batch_id: BatchId,
    pub detection_id: Option<DetectionId>,
    pub content_type: String,
    pub image: Arc<Vec<u8>>,
    pub prompt_context: Option<String>,
    pub model: Option<String>,
}

/// Raw provider output in the Talos normalized wire format (see `normalize`),
/// plus usage for cost accounting.
#[derive(Clone, Debug, Default)]
pub struct ProviderResponse {
    pub body: serde_json::Value,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub usd_estimate: Option<f64>,
}

/// Adapter for one remote (or fixture) provider. Implementations must map
/// transport failures onto `TalosError` per ADR-0007 and never fabricate output.
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;

    fn model_for(&self, _op: AiOperation) -> Option<String> {
        None
    }

    /// Fixture providers are forbidden in `live` mode routes.
    fn is_fixture(&self) -> bool {
        false
    }

    async fn call(&self, req: &ProviderRequest) -> Result<ProviderResponse, TalosError>;
}

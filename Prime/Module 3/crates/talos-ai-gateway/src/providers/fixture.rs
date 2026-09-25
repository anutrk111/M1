//! Deterministic offline provider for CI. Explicit mock, never production spoofing:
//! it is rejected in `live` mode.

use crate::provider::{Provider, ProviderRequest, ProviderResponse};
use async_trait::async_trait;
use serde_json::json;
use talos_core::TalosError;
use talos_types::AiOperation;

pub struct FixtureProvider {
    name: String,
}

impl FixtureProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Provider for FixtureProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn model_for(&self, _op: AiOperation) -> Option<String> {
        Some("fixture-v1".into())
    }

    fn is_fixture(&self) -> bool {
        true
    }

    async fn call(&self, req: &ProviderRequest) -> Result<ProviderResponse, TalosError> {
        let body = match req.operation {
            AiOperation::DetectVehiclesPlates => json!({
                "vehicles": [{
                    "label": "car", "score": 0.91,
                    "bbox": { "x_min": 0.04, "y_min": 0.16, "x_max": 0.54, "y_max": 0.88 }
                }],
                "plates": [{
                    "label": "plate", "score": 0.94,
                    "bbox": { "x_min": 0.10, "y_min": 0.40, "x_max": 0.34, "y_max": 0.46 }
                }]
            }),
            AiOperation::RunOcr => json!({
                "hypotheses": [{
                    "text": "CG04AB1234",
                    "confidence": 0.96,
                    "char_confidences": [0.97, 0.97, 0.97, 0.97, 0.97, 0.97, 0.97, 0.97, 0.97, 0.97]
                }]
            }),
            AiOperation::AnalyzeHsrp => json!({
                "score": 0.88,
                "ind_mark": "OBSERVED",
                "hologram": "OBSERVED",
                "geometry": "OBSERVED",
                "notes": ["fixture"]
            }),
            AiOperation::VlmAssist => json!({
                "confidence": 0.5,
                "observation": { "summary": "fixture visual observation" },
                "hints": {}
            }),
        };
        Ok(ProviderResponse {
            body,
            input_tokens: None,
            output_tokens: None,
            usd_estimate: None,
        })
    }
}

use serde::{Deserialize, Serialize};

/// Evidence provenance kind. Visual pipeline must only emit `VisualObservation`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    VisualObservation,
    /// Reserved for authorized external integrations (e.g. future VAHAN).
    ExternalDatabaseVerification,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub source: String,
    pub payload: serde_json::Value,
    /// Confidence in \[0.0, 1.0\].
    pub confidence: f32,
}

impl Evidence {
    pub fn visual(
        source: impl Into<String>,
        payload: serde_json::Value,
        confidence: f32,
    ) -> Self {
        Self {
            kind: EvidenceKind::VisualObservation,
            source: source.into(),
            payload,
            confidence,
        }
    }
}

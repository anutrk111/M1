use crate::evidence::Evidence;
use crate::ids::{BatchId, FrameId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DecisionOutcome {
    AutoApproved,
    SecondaryVerification,
    ReviewRequired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ReviewAction {
    Verify,
    Correct,
    Reject,
    Unreadable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FrameTerminalStatus {
    Succeeded,
    FailedValidation,
    FailedPermanent,
    FailedTransientExhausted,
    FailedNotImplemented,
    Halted,
}

/// Minimum decision JSON export / API shape (schema v2.0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionJson {
    pub schema_version: String,
    pub batch_id: BatchId,
    pub frame_id: FrameId,
    pub plate_text: String,
    pub grammar_ok: bool,
    pub hsrp_score: f32,
    pub fused_confidence: f32,
    pub outcome: DecisionOutcome,
    pub evidence: Vec<Evidence>,
    /// Stage timings in milliseconds, keyed by stage index `"0"`..`"7"`.
    pub stages_ms: BTreeMap<String, u64>,
}

impl DecisionJson {
    pub fn stages_ms_from_array(timings: &[u64; 8]) -> BTreeMap<String, u64> {
        timings
            .iter()
            .enumerate()
            .map(|(i, ms)| (i.to_string(), *ms))
            .collect()
    }
}

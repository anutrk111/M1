//! Frozen cross-module contracts for decision, review, audit, export, and access.

use crate::artifacts::{CueObservation, FusedDecision};
use crate::decision::{DecisionOutcome, ReviewAction};
use crate::ids::{
    AuditEventId, BatchId, ConfigRevisionId, DetectionId, FrameId, ReviewEventId, TraceId, UserId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Downstream machine verdict for one detection (M08 output, immutable under M09).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MachineOutcome {
    Accept,
    ReviewRequired,
    RejectUnusable,
}

impl From<&DecisionOutcome> for MachineOutcome {
    /// Stage 7 bands map conservatively: anything short of auto-approval needs review.
    fn from(value: &DecisionOutcome) -> Self {
        match value {
            DecisionOutcome::AutoApproved => Self::Accept,
            DecisionOutcome::SecondaryVerification | DecisionOutcome::ReviewRequired => {
                Self::ReviewRequired
            }
        }
    }
}

/// Immutable machine decision (ADR-0024). Review never mutates this record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachineDecision {
    pub batch_id: BatchId,
    pub frame_id: FrameId,
    pub detection_id: DetectionId,
    pub outcome: MachineOutcome,
    /// Fused confidence in \[0.0, 1.0\].
    pub confidence: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plate_text: Option<String>,
    pub reasons: Vec<String>,
    /// Opaque references to evidence / artifacts supporting this decision.
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_revision_id: Option<ConfigRevisionId>,
    pub decided_at: DateTime<Utc>,
}

impl MachineDecision {
    /// Project Stage 7 fusion state into the frozen downstream contract.
    pub fn from_fused(
        batch_id: BatchId,
        frame_id: FrameId,
        detection_id: DetectionId,
        fused: &FusedDecision,
        evidence_refs: Vec<String>,
        decided_at: DateTime<Utc>,
    ) -> Self {
        Self {
            batch_id,
            frame_id,
            detection_id,
            outcome: MachineOutcome::from(&fused.outcome),
            confidence: fused.fused_confidence,
            plate_text: (!fused.plate_text.is_empty()).then(|| fused.plate_text.clone()),
            reasons: fused.hard_fail_reasons.clone(),
            evidence_refs,
            config_revision_id: fused.config_revision_id.clone(),
            decided_at,
        }
    }
}

/// Operational roles (ADR-0043). Authorization still checks [`Permission`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    SuperAdmin,
    Clerk,
    ReviewingOfficer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Permission {
    #[serde(rename = "intake.submit")]
    IntakeSubmit,
    #[serde(rename = "review.read")]
    ReviewRead,
    #[serde(rename = "review.act")]
    ReviewAct,
    #[serde(rename = "review.override")]
    ReviewOverride,
    #[serde(rename = "export.read")]
    ExportRead,
    #[serde(rename = "export.audit")]
    ExportAudit,
    #[serde(rename = "user.manage")]
    UserManage,
    #[serde(rename = "config.read")]
    ConfigRead,
    #[serde(rename = "config.write")]
    ConfigWrite,
}

impl Role {
    /// Default grants. Deployments may override via `configs/auth.toml`.
    /// `SuperAdmin` has no automatic review authority.
    pub fn default_permissions(self) -> &'static [Permission] {
        use Permission::*;
        match self {
            Role::SuperAdmin => &[UserManage, ConfigRead, ConfigWrite, ExportRead, ExportAudit],
            Role::Clerk => &[IntakeSubmit, ReviewRead, ReviewAct, ExportRead],
            Role::ReviewingOfficer => &[ReviewRead, ReviewAct, ReviewOverride, ExportRead],
        }
    }
}

/// Append-only human review event (M09).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewEvent {
    pub review_event_id: ReviewEventId,
    pub batch_id: BatchId,
    pub frame_id: FrameId,
    pub detection_id: DetectionId,
    pub actor: UserId,
    pub role: Role,
    pub action: ReviewAction,
    /// Required when `action == Correct` (ADR-0025).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corrected_plate_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub at: DateTime<Utc>,
}

/// Effective status after machine decision and any review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinalStatus {
    Accepted,
    Corrected,
    Rejected,
    Unreadable,
    PendingReview,
}

/// Effective result for one detection: machine view + human overlay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewedResult {
    pub batch_id: BatchId,
    pub frame_id: FrameId,
    pub detection_id: DetectionId,
    pub machine_outcome: MachineOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine_plate_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_plate_text: Option<String>,
    pub final_status: FinalStatus,
    pub review_event_ids: Vec<ReviewEventId>,
}

/// Append-only audit event (any module).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub audit_event_id: AuditEventId,
    pub at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<UserId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// Dotted action name, e.g. `review.correct`, `config.activate`.
    pub action: String,
    pub subject_kind: String,
    pub subject_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<TraceId>,
    #[serde(default)]
    pub details: serde_json::Value,
}

/// One export row (M11). Carries lineage and status, not only the plate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportRow {
    pub batch_id: BatchId,
    pub frame_id: FrameId,
    /// Original source path / key from intake.
    pub image: String,
    pub detection_id: DetectionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plate_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_confidence: Option<f32>,
    pub hsrp_status: CueObservation,
    pub machine_outcome: MachineOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human_action: Option<ReviewAction>,
    pub final_status: FinalStatus,
    pub timestamp: DateTime<Utc>,
}

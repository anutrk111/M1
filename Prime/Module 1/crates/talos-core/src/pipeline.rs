use crate::error::TalosError;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use talos_types::*;
use tracing::{info_span, Instrument};

/// Pipeline stage index 0..=7.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StageId {
    ImageQuality = 0,
    Detection = 1,
    Rectification = 2,
    Ocr = 3,
    Grammar = 4,
    Hsrp = 5,
    ObservationDedup = 6,
    Decision = 7,
}

impl StageId {
    pub const ALL: [StageId; 8] = [
        Self::ImageQuality,
        Self::Detection,
        Self::Rectification,
        Self::Ocr,
        Self::Grammar,
        Self::Hsrp,
        Self::ObservationDedup,
        Self::Decision,
    ];

    pub fn index(self) -> u8 {
        self as u8
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::ImageQuality => "image_quality",
            Self::Detection => "detection",
            Self::Rectification => "rectification",
            Self::Ocr => "ocr",
            Self::Grammar => "grammar",
            Self::Hsrp => "hsrp",
            Self::ObservationDedup => "observation_dedup",
            Self::Decision => "decision",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageStatus {
    Continue,
    SkipRemaining,
    Halt,
}

/// Mutable per-frame working state flowing through Stages 0–7.
#[derive(Clone, Debug)]
pub struct FrameContext {
    pub intake: IntakeEnvelope,
    pub trace_id: TraceId,
    pub quality: Option<ImageQualityReport>,
    pub detections: Option<Detections>,
    pub rectified: Option<RectifiedPlate>,
    pub ocr: Vec<OcrHypothesis>,
    pub grammar: Option<GrammarResult>,
    pub hsrp: Option<HsrpEvidence>,
    /// Stage 6 observation cluster / dedup result (not vehicle tracking).
    pub observation_dedup: Option<DedupResult>,
    pub fused: Option<FusedDecision>,
    pub evidence: Vec<Evidence>,
    pub timings_ms: [u64; 8],
    pub terminal_status: Option<FrameTerminalStatus>,
}

impl FrameContext {
    pub fn new(intake: IntakeEnvelope, trace_id: TraceId) -> Self {
        Self {
            intake,
            trace_id,
            quality: None,
            detections: None,
            rectified: None,
            ocr: Vec::new(),
            grammar: None,
            hsrp: None,
            observation_dedup: None,
            fused: None,
            evidence: Vec::new(),
            timings_ms: [0; 8],
            terminal_status: None,
        }
    }

    /// Append visual evidence only. External DB evidence is rejected.
    pub fn push_visual_evidence(&mut self, evidence: Evidence) -> Result<(), TalosError> {
        if evidence.kind != EvidenceKind::VisualObservation {
            return Err(TalosError::Validation(
                "visual pipeline may only append VisualObservation evidence".into(),
            ));
        }
        self.evidence.push(evidence);
        Ok(())
    }

    pub fn to_decision_json(&self) -> Result<DecisionJson, TalosError> {
        let fused = self.fused.as_ref().ok_or_else(|| {
            TalosError::Internal("decision JSON requested before Stage 7 fusion".into())
        })?;
        Ok(DecisionJson {
            schema_version: DECISION_SCHEMA_VERSION.to_owned(),
            batch_id: self.intake.batch_id.clone(),
            frame_id: self.intake.frame_id.clone(),
            plate_text: fused.plate_text.clone(),
            grammar_ok: fused.grammar_ok,
            hsrp_score: fused.hsrp_score,
            fused_confidence: fused.fused_confidence,
            outcome: fused.outcome.clone(),
            evidence: self.evidence.clone(),
            stages_ms: DecisionJson::stages_ms_from_array(&self.timings_ms),
        })
    }
}

#[async_trait]
pub trait Stage: Send + Sync {
    fn id(&self) -> StageId;
    fn name(&self) -> &'static str;
    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError>;
}

/// Ordered Stage 0–7 orchestrator.
pub struct Pipeline {
    stages: Vec<Arc<dyn Stage>>,
}

impl Pipeline {
    /// Build a pipeline. Stages must be unique and sorted by `StageId` order.
    pub fn new(stages: Vec<Arc<dyn Stage>>) -> Result<Self, TalosError> {
        if stages.is_empty() {
            return Err(TalosError::Internal("pipeline has no stages".into()));
        }
        for window in stages.windows(2) {
            if window[0].id().index() >= window[1].id().index() {
                return Err(TalosError::Internal(
                    "pipeline stages must be strictly increasing by StageId".into(),
                ));
            }
        }
        Ok(Self { stages })
    }

    pub fn stages(&self) -> &[Arc<dyn Stage>] {
        &self.stages
    }

    /// Run stages in order. Records timings. Maps terminal status on Halt / NotImplemented.
    #[tracing::instrument(
        name = "frame.run",
        skip(self, ctx),
        fields(
            trace_id = %ctx.trace_id,
            batch_id = %ctx.intake.batch_id,
            frame_id = %ctx.intake.frame_id,
        )
    )]
    pub async fn run(&self, ctx: &mut FrameContext) -> Result<(), TalosError> {
        for stage in &self.stages {
            let stage_id = stage.id();
            let span = info_span!(
                "stage.process",
                stage_id = stage_id.index(),
                stage_name = stage.name()
            );

            let started = Instant::now();
            let result = stage.process(ctx).instrument(span).await;
            ctx.timings_ms[stage_id.index() as usize] = started.elapsed().as_millis() as u64;

            match result {
                Ok(StageStatus::Continue) => continue,
                Ok(StageStatus::SkipRemaining) => {
                    tracing::info!(stage = stage_id.index(), "skip remaining stages");
                    return Ok(());
                }
                Ok(StageStatus::Halt) => {
                    ctx.terminal_status = Some(FrameTerminalStatus::Halted);
                    tracing::warn!(stage = stage_id.index(), "pipeline halted");
                    return Ok(());
                }
                Err(TalosError::NotImplemented(msg)) => {
                    ctx.terminal_status = Some(FrameTerminalStatus::FailedNotImplemented);
                    return Err(TalosError::NotImplemented(msg));
                }
                Err(TalosError::Validation(msg)) => {
                    ctx.terminal_status = Some(FrameTerminalStatus::FailedValidation);
                    return Err(TalosError::Validation(msg));
                }
                Err(TalosError::Permanent(msg)) => {
                    ctx.terminal_status = Some(FrameTerminalStatus::FailedPermanent);
                    return Err(TalosError::Permanent(msg));
                }
                Err(TalosError::Transient(msg)) => {
                    return Err(TalosError::Transient(msg));
                }
                Err(other) => return Err(other),
            }
        }

        if ctx.fused.is_some() {
            ctx.terminal_status = Some(FrameTerminalStatus::Succeeded);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingStage {
        id: StageId,
        status: StageStatus,
        counter: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Stage for CountingStage {
        fn id(&self) -> StageId {
            self.id
        }
        fn name(&self) -> &'static str {
            "counting"
        }
        async fn process(&self, _ctx: &mut FrameContext) -> Result<StageStatus, TalosError> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(self.status)
        }
    }

    fn sample_ctx() -> FrameContext {
        let intake = IntakeEnvelope::new(
            BatchId::new("batch"),
            FrameId::new("frame"),
            SourceRef {
                kind: SourceKind::Folder,
                path_or_key: "img.jpg".into(),
            },
            ImageRef {
                content_type: "image/jpeg".into(),
                sha256: "abc".into(),
                bytes_ref: "file://img.jpg".into(),
            },
        );
        FrameContext::new(intake, TraceId::new("trace"))
    }

    #[tokio::test]
    async fn runs_stages_in_order() {
        let counter = Arc::new(AtomicUsize::new(0));
        let stages: Vec<Arc<dyn Stage>> = StageId::ALL
            .into_iter()
            .map(|id| {
                Arc::new(CountingStage {
                    id,
                    status: StageStatus::Continue,
                    counter: counter.clone(),
                }) as Arc<dyn Stage>
            })
            .collect();
        let pipeline = Pipeline::new(stages).unwrap();
        let mut ctx = sample_ctx();
        pipeline.run(&mut ctx).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 8);
    }

    #[tokio::test]
    async fn halt_stops_pipeline() {
        let counter = Arc::new(AtomicUsize::new(0));
        let stages: Vec<Arc<dyn Stage>> = vec![
            Arc::new(CountingStage {
                id: StageId::ImageQuality,
                status: StageStatus::Halt,
                counter: counter.clone(),
            }),
            Arc::new(CountingStage {
                id: StageId::Detection,
                status: StageStatus::Continue,
                counter: counter.clone(),
            }),
        ];
        let pipeline = Pipeline::new(stages).unwrap();
        let mut ctx = sample_ctx();
        pipeline.run(&mut ctx).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(ctx.terminal_status, Some(FrameTerminalStatus::Halted));
    }

    #[tokio::test]
    async fn rejects_external_evidence_on_visual_path() {
        let mut ctx = sample_ctx();
        let bad = Evidence {
            kind: EvidenceKind::ExternalDatabaseVerification,
            source: "vahan".into(),
            payload: serde_json::json!({}),
            confidence: 1.0,
        };
        assert!(ctx.push_visual_evidence(bad).is_err());
    }
}

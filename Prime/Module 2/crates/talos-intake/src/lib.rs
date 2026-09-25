//! M02 Batch & File Intake — folders / ZIP / uploads → M01 `IntakeEnvelope` + `BatchManifest`.
//!
//! Does **not** run OCR, detection, HSRP, decisions, or real durable queues (M10).

mod config;
mod discover;
mod image;
mod metadata;
mod pipeline;
mod recovery;
mod sink;
mod staging;
mod types;
mod upload;
mod zip_safe;

pub use config::{load_intake_config, IntakeConfig, StagingPolicy};
pub use pipeline::{
    import_folder, import_source, import_upload, import_zip, ImportResult, IntakeSource,
};
pub use recovery::{RecoveryContext, RecoveryKey};
pub use sink::{
    DurableSink, DurableSinkAdapter, FailingFrameSink, FrameSink, IdempotencyKey,
    InMemoryDurableSink, InMemoryFrameSink, SinkAck, UnconfiguredDurableSink,
};
pub use staging::{plan_cleanup, CleanupEntry, CleanupPlan, CleanupReason};
pub use types::{
    BatchCounts, BatchManifest, BatchOutcome, DuplicateReference, FrameRecord, ImportRequest,
    IntakeStatus, MetadataRecord,
};
pub use upload::UploadedFile;

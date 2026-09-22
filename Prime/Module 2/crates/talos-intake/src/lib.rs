//! M02 Batch & File Intake — folders / ZIP → M01 `IntakeEnvelope` + `BatchManifest`.
//!
//! Does **not** run OCR, detection, HSRP, decisions, or durable queues.

mod config;
mod discover;
mod image;
mod metadata;
mod pipeline;
mod sink;
mod types;
mod zip_safe;

pub use config::{load_intake_config, IntakeConfig};
pub use pipeline::{import_folder, import_zip, ImportResult};
pub use sink::{FailingFrameSink, FrameSink, InMemoryFrameSink};
pub use types::{
    BatchCounts, BatchManifest, BatchOutcome, DuplicateReference, FrameRecord, ImportRequest,
    IntakeStatus, MetadataRecord,
};

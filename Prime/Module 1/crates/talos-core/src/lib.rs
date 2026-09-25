//! M01 Foundation & Core — pipeline orchestration, errors, runtime, utilities.
//!
//! See `docs/pdr/M01-foundation-and-core.md`.

pub mod backend;
pub mod error;
pub mod fixture;
pub mod health;
pub mod pipeline;
pub mod ports;
pub mod runtime;
pub mod util;

pub use error::TalosError;
pub use pipeline::{FrameContext, Pipeline, Stage, StageId, StageStatus};
pub use ports::{OcrConsensus, VisionGateway, VisionRequest, VlmAssistResult, VlmRequest};
pub use talos_config;
pub use talos_types;

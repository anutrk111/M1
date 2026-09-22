//! Metric and log field naming conventions for Talos-RS (M01).
//!
//! Other modules register concrete series using these names; this crate does
//! not bind a metrics backend.

/// Histogram: stage latency. Labels: `stage_id`.
pub const STAGE_DURATION_SECONDS: &str = "talos_stage_duration_seconds";

/// Counter: completed frames. Labels: `outcome` or `terminal_status`.
pub const PIPELINE_FRAMES_TOTAL: &str = "talos_pipeline_frames_total";

/// Counter: errors. Labels: `class`, `stage`.
pub const ERRORS_TOTAL: &str = "talos_errors_total";

/// Counter: not-implemented hits. Labels: `component`.
pub const NOT_IMPLEMENTED_TOTAL: &str = "talos_not_implemented_total";

/// Required log / span field names.
pub mod fields {
    pub const TRACE_ID: &str = "trace_id";
    pub const BATCH_ID: &str = "batch_id";
    pub const FRAME_ID: &str = "frame_id";
    pub const STAGE_ID: &str = "stage_id";
    pub const SCHEMA_VERSION: &str = "schema_version";
}

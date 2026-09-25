//! M01 common types and schemas (no I/O, no Tokio).
//!
//! These are the frozen cross-module contracts (ADR-0042). Changes go through
//! an `impl/m01-contracts` PR.

mod artifacts;
mod cost;
mod decision;
mod error;
mod evidence;
mod governance;
mod ids;
mod intake;

pub use artifacts::*;
pub use cost::*;
pub use decision::*;
pub use error::*;
pub use evidence::*;
pub use governance::*;
pub use ids::*;
pub use intake::*;

/// Intake envelope schema version (M02 output).
pub const INTAKE_SCHEMA_VERSION: &str = "2.0";

/// Decision / export schema version. 2.1: normalized bbox, ternary HSRP,
/// required `DetectionId` on downstream artifacts, `GrammarStatus`.
pub const DECISION_SCHEMA_VERSION: &str = "2.1";

/// Backward-compatible alias for the intake envelope schema version.
pub const SCHEMA_VERSION: &str = INTAKE_SCHEMA_VERSION;

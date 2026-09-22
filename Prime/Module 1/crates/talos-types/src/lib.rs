//! M01 common types and schemas (no I/O, no Tokio).

mod artifacts;
mod decision;
mod evidence;
mod ids;
mod intake;

pub use artifacts::*;
pub use decision::*;
pub use evidence::*;
pub use ids::*;
pub use intake::*;

/// JSON schema version for intake and decision envelopes.
pub const SCHEMA_VERSION: &str = "2.0";

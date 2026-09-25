//! Cross-module integration gates. All content lives in `tests/`; this crate ships no code.
//!
//! - G1 (`tests/g1_intake_gateway.rs`): M02 intake → M01 envelope/port → M03 gateway
//!   (fixture provider), asserting frozen `talos-types` contracts and lineage.

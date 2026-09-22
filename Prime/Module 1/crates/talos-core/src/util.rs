//! Hashing, ID generation, and time helpers.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use talos_types::{BatchId, FrameId, TraceId};
use ulid::Ulid;

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

pub fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

pub fn new_batch_id() -> BatchId {
    BatchId::new(Ulid::new().to_string())
}

pub fn new_frame_id() -> FrameId {
    FrameId::new(Ulid::new().to_string())
}

pub fn new_trace_id() -> TraceId {
    TraceId::new(Ulid::new().to_string())
}

pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_empty_vector() {
        let hex = sha256_hex(b"");
        assert_eq!(
            hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_ne!(hex, blake3_hex(b""));
    }

    #[test]
    fn ids_are_non_empty() {
        assert!(!new_batch_id().as_str().is_empty());
        assert!(!new_frame_id().as_str().is_empty());
        assert!(!new_trace_id().as_str().is_empty());
    }
}

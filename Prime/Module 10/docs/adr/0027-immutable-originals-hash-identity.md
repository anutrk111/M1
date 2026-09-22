# ADR 0027 — Immutable Originals; Hash Is Identity; Logical Keys for Humans

## Status

Accepted

## Context

Paths alone are weak evidence identity (rename, overwrite, collision). Auditors also need human-readable locations for debugging.

## Decision

1. Original frames are **write-once**; never overwrite in place (aligns with M04 immutable originals).
2. Integrity identity is **content hash** (`sha256:<digest>`). The path is **not** evidence identity.
3. Hybrid addressing:
   - Logical key: `{batch_id}/{frame_id}/{detection_id}/{artifact_kind}`
   - Integrity: `sha256:<digest>`
4. DB rows join to objects via hash + logical key metadata (immutable lineage).

## Consequences

- Tamper detection and dedup by hash.
- Operators can navigate logical keys without treating path as proof.

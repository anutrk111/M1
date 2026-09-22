# ADR 0028 — Storage Ports, Not Drivers, in Upstream Modules

## Status

Accepted

## Context

If M02/M04/M08/M09 call MinIO/S3/Postgres clients directly, swapping backends or fixture mode becomes invasive.

## Decision

1. Upstream modules depend only on storage **ports**:
   - `FrameStore`
   - `BlobStore`
   - `DecisionStore`
   - `ReviewEventStore`
   - `DedupClusterStore`
2. M10 implements adapters (Postgres, MinIO/S3, fixture FS/SQLite).
3. M09 does not reach object-store internals; it emits ReviewEvents through `ReviewEventStore`.
4. M10 does **not** decide correction validity or reviewer authorization.

```text
M09 → ReviewEventStore → M10
M08 → DecisionStore    → M10
M04 → BlobStore        → M10
M02 → FrameStore/BlobStore → M10
```

## Consequences

- Hexagonal boundary preserved.
- Fixture CI path without cloud credentials.

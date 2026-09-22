# ADR 0032 — Export Via M10 Ports Only

## Status

Accepted

## Context

Direct Postgres/MinIO access from export code bypasses ArtifactState, hash identity, and port boundaries.

## Decision

1. M11 reads only through M10 ports (`DecisionStore`, `ReviewEventStore`, `FrameStore`, blob metadata as needed).
2. M11 never mutates stored decisions, review events, or blobs.
3. Authorization to call export APIs is M12 (`export.read` / `export.audit`); meaning of export rows is M11.

## Consequences

- Storage adapters can change without rewriting export queries.
- Incomplete artifacts remain governed by M10 ArtifactState + M11 availability policy.

# ADR 0026 — PostgreSQL System of Record; Timescale Metrics; Object Store Blobs

## Status

Accepted

## Context

Talos needs relational metadata, time-series observability, and large binary artifacts. One store for all concerns creates coupling and operational risk.

## Decision

1. **PostgreSQL** is the relational system of record: batches, frames, DetectionId indexes, MachineDecision, ReviewEvent, ReviewedResult, ArtifactState metadata.
2. **TimescaleDB** (or Postgres tables until enabled) holds stage latency / metrics hypertables.
3. **MinIO/S3-compatible** object store holds original frames and derived images/blobs.
4. M10 owns adapters; upstream modules use ports (ADR-0028).

## Consequences

- Clear ownership per data class.
- Object store can be swapped without rewriting M02/M04/M08/M09.

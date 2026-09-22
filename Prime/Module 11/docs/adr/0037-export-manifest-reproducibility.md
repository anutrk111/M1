# ADR 0037 — ExportManifest and Reproducible Ordering

## Status

Accepted

## Context

Without a manifest and stable ordering, the same filters can yield non-comparable files across runs (pagination drift, silent row drops).

## Decision

1. Every generated export file is paired with an `ExportManifest` containing at least:
   - `export_id`
   - generation timestamp (UTC)
   - requested view
   - filters
   - schema version
   - row count
   - output SHA-256
2. Chunking/pagination is deterministic; row order is stable for the same snapshot + filter (config `order_by`).
3. Manifest is part of the deliverable set (sidecar JSON or documented companion object).

## Consequences

- Auditors can re-verify file integrity and parameters.
- Large exports remain comparable across regenerations of the same snapshot.

# ADR 0033 — Export Formats CSV / XLSX / JSONL

## Status

Accepted

## Context

Transport admins need spreadsheet-friendly files; systems need schema-stable machine-readable dumps.

## Decision

1. Primary formats: **CSV** and **XLSX**.
2. Optional **JSONL** for schema v2.0 DecisionJson-shaped rows.
3. Summary reports (counts by outcome, review action, etc.) are first-class export products, still presentation-only.
4. Lineage columns always include `batch_id`, `frame_id`, and `DetectionId` when plate-scoped.

## Consequences

- One module owns format contracts.
- Spreadsheet safety rules (ADR-0038) apply to CSV/XLSX cell serialization.

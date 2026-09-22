# ADR 0011 — Multi-Plate Talos DetectionId and Geometry Canon

## Status

Accepted

## Context

Lineage from detection through rectification and OCR/HSRP must not break when providers or fallbacks change. Coordinate systems differ across providers. Overlap boxes and cross-frame dedup are different problems.

## Decision

1. **Talos-owned `DetectionId`** for every accepted vehicle/plate; `provider_detection_id` optional provenance only.
2. Output geometry is **original-frame pixel** `BoundingBox` (`x,y,width,height` with in-bounds rules). Normalize at M03/M05 boundary so M04 never sees provider-normalized coords.
3. Support **0..N** plates; architecture not locked to a single best plate.
4. Optional `associated_vehicle_id` is spatial only; never drop a valid plate solely for missing association.
5. In-frame **detection suppression** (`[dedup]` IoU) ≠ M01 Stage 6 pipeline deduplication.
6. Record `DetectionSummary` (raw/accepted/filtered/suppressed/truncated) — no silent truncation.
7. Provider success with 0 plates → Continue with empty list; not Validation; not final rejection.
8. M05 must not emit plate text, registration codes, HSRP flags, or decisions.

## Consequences

- M04 Stage 2 preserves the same `DetectionId` on `RectifiedPlate`.
- M06/M07/M08 join evidence by `DetectionId`.
- Metrics can separate empty frames from provider failures.

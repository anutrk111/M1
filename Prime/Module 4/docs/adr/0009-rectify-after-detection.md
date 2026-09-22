# ADR 0009 — Rectify After Detection (Multi-Plate + Provenance)

## Status

Accepted

## Context

Plate crops require Stage 1 detections. Architecture must support multiple visible plates while defaulting to `top_k = 1`. Enhancement must remain auditable and must not mutate the original frame.

## Decision

1. Stage 2 runs **only after** Stage 1; never before detection.
2. Output type is `RectificationResult::{ Plates(Vec<RectifiedPlate>), NoPlateDetected }` — not a single best-plate-only type.
3. Process at most `top_k` plates (default 1), score-ordered.
4. Keep distinct refs: original crop, optional enhanced crop, optional API-prepared image; hash derived artifacts when configured.
5. Record `TransformProvenance` (step names + parameters).
6. Zero detections → `NoPlateDetected` + Continue; **not** an M04 validation error.
7. Prefer plate crop / api-prepared bytes for M03 plate-only operations.
8. Never invent OCR characters; never overwrite M02 original bytes/hash.

## Consequences

- M01 `FrameContext` must store `rectification: RectificationResult`.
- M08/M09 can reason about no-plate vs preprocess failure separately.
- Review can compare original vs enhanced appearance.

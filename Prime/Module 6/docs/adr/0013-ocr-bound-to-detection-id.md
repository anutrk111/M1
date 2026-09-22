# ADR 0013 — OCR Hypothesis Bound to DetectionId

## Status

Accepted

## Context

Treating OCR output as a plain string breaks lineage, audit, and evidence-supported grammar corrections (e.g. `O↔0`).

## Decision

1. An OCR hypothesis is a **structured object**, not a bare string.
2. Every hypothesis MUST include at least:
   - `detection_id` (Talos-owned, same as RectifiedPlate)
   - `source_artifact` (which crop representation was read)
   - provider / model **provenance**
   - `raw_text`
   - `normalized_text` (light pre-grammar normalize only when documented)
   - overall `confidence`
   - character-level evidence **when the provider supports it** (optional; absence must not be fabricated)
3. Losing `DetectionId` on any hypothesis is a contract violation.
4. Multi-plate frames produce one hypothesis set per `DetectionId`.

## Consequences

- M08 and review can join OCR to the originating detection.
- Grammar (ADR-0019) can require character/alternative evidence before corrections.

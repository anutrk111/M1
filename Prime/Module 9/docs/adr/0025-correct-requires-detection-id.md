# ADR 0025 — Correct Field Contract; No DetectionId Mutation

## Status

Accepted

## Context

Limiting Correct to a single corrected plate string loses previous value, officer context, and lineage. Allowing DetectionId changes or sibling deletion would break M05–M08 provenance.

## Decision

1. A `Correct` action MUST carry at least:
   - `previous_value`
   - `corrected_value`
   - `DetectionId` (unchanged identity of the plate under review)
   - officer identity
   - timestamp
   - reason / note
2. Correction **content** may be recorded as `VisualObservation` (what was corrected).
3. Officer identity / action / audit metadata live in **ReviewEvent provenance**, not mixed into the evidence payload.
4. M09 **forbids**: changing `DetectionId`, minting a new detection, deleting sibling candidates from provenance.

## Consequences

- Corrections remain joinable to the originating detection.
- EvidenceKind stays clean; audit stays in ReviewEvent.

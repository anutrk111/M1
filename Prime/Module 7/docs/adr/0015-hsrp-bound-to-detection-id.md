# ADR 0015 — HSRP Observations Bound to DetectionId

## Status

Accepted

## Context

HSRP cues must join OCR/grammar and decisions on the same plate lineage.

## Decision

1. Every HSRP observation record MUST include:
   - `detection_id` (Talos-owned, same as RectifiedPlate)
   - `source_artifact`
   - observation type / cue name
   - observability value (see ADR-0020)
   - confidence (when meaningful)
   - provider / model provenance
2. Losing `DetectionId` is a contract violation.
3. No plate → no HSRP call; empty evidence for that id (not fabricated negatives).

## Consequences

- M08 can fuse HSRP with OCR/grammar per DetectionId.
- Review can audit which crop produced which cue.

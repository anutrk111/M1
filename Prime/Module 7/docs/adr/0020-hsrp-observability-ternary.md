# ADR 0020 — HSRP Observability Ternary

## Status

Accepted

## Context

Boolean `false` for “IND mark” collapses two different facts: the cue was not present, versus the image could not show the cue (crop, blur, occlusion, angle).

## Decision

1. Cue fields use ternary (or equivalent) semantics, not forced booleans:
   - `Observed` — cue appears present in this image
   - `NotObserved` — cue appears absent **and** the region was observable
   - `NotObservable` / `Unknown` — image cannot support a present/absent claim
2. **“Not visible in this image” ≠ “not present on the plate.”**
3. Config `allow_boolean_collapse = false` by default; do not silently coerce Unknown → false.
4. Downstream (M08) may treat Unknown as non-confirming evidence, never as hard negative by default.

## Consequences

- Safer audit language for HSRP cues.
- Avoids false “non-compliant” pressure from unreadable crops.

# ADR 0008 — Stage 0 Quality Gates (Accept / Degraded / Unusable)

## Status

Accepted

## Context

Binary pass/fail quality gates discard AI-readable but degraded frames too early. M04 Stage 0 must contribute **evidence**, not final rejection, except for hard undecodable inputs.

## Decision

1. Quality class ∈ { `Accept`, `Degraded`, `Unusable` }.
2. Soft issues (blur, darkness, marginal resolution) → `Degraded` + pipeline **Continue** when `on_soft_fail = "continue"` (default).
3. Only undecodable / corrupt / unsupported images → `Unusable` + **Halt**.
4. Final Auto/Secondary/Review remains **M08**; Stage 0 alone must not final-reject on soft metrics.
5. Emit `VisualObservation` evidence with source `preprocess.s0`.

## Consequences

- Blurred-but-readable plates still reach detection/OCR.
- Review and fusion can weight `Degraded` without losing the frame.

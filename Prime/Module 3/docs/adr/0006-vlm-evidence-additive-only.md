# ADR 0006 — VLM Evidence Additive Only

## Status

Accepted

## Context

M01 requires evidence-driven decisions and forbids conflating VLM assistance with deterministic grammar or external registry verification. M03 executes `vlm_assist` on behalf of M08.

## Decision

1. `vlm_assist` must return `Evidence` with `kind = VisualObservation` only.
2. `source` naming: `vlm.<provider>.<model>`.
3. M03 must not mutate `GrammarResult`, decision thresholds, or hard-fail flags.
4. Optional `hints` JSON is non-authoritative input to M08 fusion/review — never a silent override.
5. M03 must not write `ExternalDatabaseVerification`.

## Consequences

- Review UI and audit can show VLM as observational evidence.
- Grammar and configured hard-fails remain authoritative unless a human (M09) corrects.

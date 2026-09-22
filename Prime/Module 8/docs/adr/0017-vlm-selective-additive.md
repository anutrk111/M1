# ADR 0017 — VLM Selective and Additive Only

## Status

Accepted

## Context

VLM assist can help on ambiguous bands but must not override grammar hard-fails or silently raise outcomes past thresholds.

## Decision

1. Call M03 `vlm_assist` only when configured trigger matches (e.g. secondary band).
2. VLM evidence is **additive** `VisualObservation` only (ADR-0006 / EvidenceKind rules).
3. VLM **cannot** bypass decision thresholds (`can_bypass_thresholds = false`).
4. VLM **cannot** erase hard evidence (grammar hard-fail, NoPlateDetected hard-fail, etc.).
5. VLM never becomes ExternalDatabaseVerification.

## Consequences

- Ambiguous cases can gain explanatory evidence without corrupting authority.
- Review still sees grammar/HSRP/OCR hard constraints.

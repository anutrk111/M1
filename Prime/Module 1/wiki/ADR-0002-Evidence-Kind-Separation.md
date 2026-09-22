# ADR 0002 — Evidence Kind Separation

## Status

Accepted

## Context

PDR requires evidence-driven decisions and forbids conflating visual ANPR outputs with external registry lookups (e.g. future VAHAN).

## Decision

1. `EvidenceKind` has exactly two variants at M01:
   - `VisualObservation` — vision, OCR, HSRP, VLM, human visual review notes.
   - `ExternalDatabaseVerification` — reserved for authorized integration adapters only.
2. `FrameContext::push_visual_evidence` rejects non-visual kinds.
3. VLM / secondary AI may only append `VisualObservation` evidence and must not silently override deterministic grammar or hard-fail flags.

## Consequences

- Visual Stages 0–7 never write `ExternalDatabaseVerification`.
- VAHAN-class adapters (when added) use a separate API path to attach external evidence.
- Export and audit can distinguish observational vs registry-backed claims.

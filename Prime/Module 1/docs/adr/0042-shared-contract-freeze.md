# ADR 0042 — Shared Contract Freeze (Gate G0)

## Status

Accepted

## Context

M02–M12 will be implemented in parallel lanes by different agents. If modules can redefine or silently reshape shared types, lanes break each other and lineage is lost.

## Decision

1. `talos-types` (M01) is the single authority for cross-module contracts. No module redefines these types:
   - IDs: `BatchId`, `FrameId`, `DetectionId`, `ConfigRevisionId`, `UserId`, `ReviewEventId`, `AuditEventId`, `ExportJobId`
   - Geometry: `BoundingBox`, `PixelRect`
   - Artifacts: `Detections`, `RectifiedPlate`, `OcrHypothesis`, `GrammarStatus`/`GrammarResult`, `OcrResult`, `CueObservation`/`HsrpEvidence`
   - Decision and review: `MachineOutcome`/`MachineDecision`, `ReviewAction`, `ReviewEvent`, `FinalStatus`/`ReviewedResult`, `AuditEvent`
   - Export, access and accounting: `ExportRow`, `Role`/`Permission`, `CostEvent`/`AiOperation`
   - Terminal states: `FrameTerminalStatus`
2. **Lineage required:** `RectifiedPlate`, `OcrHypothesis`, `GrammarResult`, `OcrResult`, `HsrpEvidence`, `MachineDecision`, `ReviewEvent`, `ReviewedResult` and `ExportRow` carry a non-optional `DetectionId`. Only raw `Detection` (pre-minting) keeps it optional.
3. **HSRP** cues use `CueObservation { OBSERVED, NOT_OBSERVED, NOT_OBSERVABLE }` ([ADR-0020](../../../Module%207/docs/adr/0020-hsrp-observability-ternary.md)). Booleans are rejected.
4. **Schema versions:** `INTAKE_SCHEMA_VERSION = "2.0"` (unchanged), `DECISION_SCHEMA_VERSION = "2.1"`.
5. **Change control:** any contract change lands through an `impl/m01-contracts` PR with updated golden fixtures in `talos-types/tests/fixtures/`. Lane branches must not edit `talos-types` directly.
6. **Legacy payloads:** 2.0-era JSON without `detection_id`, with boolean HSRP flags, or with `{x,y,w,h}` boxes fails deserialization. There is no silent defaulting. Migration is an explicit offline transform (none exists in production yet, so none is shipped).
7. `talos-types` has no dependency on `talos-core`; invariant violations return `ContractError`, which `talos-core` maps to `TalosError::Validation`.

## Consequences

- Parallel lanes compile against one frozen surface; the golden tests are the tripwire.
- `FusedDecision` remains Stage 7 working state; `MachineDecision::from_fused` projects it into the downstream contract.

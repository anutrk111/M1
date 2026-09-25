# M01 Contract Consolidation

**Status:** Frozen at Gate G0 ([ADR-0042](../adr/0042-shared-contract-freeze.md)).
**Rule:** contract changes land only through `impl/m01-contracts` PRs with updated golden fixtures (`crates/talos-types/tests/fixtures/`).

## Classification (post-G0)

| Contract | State |
|----------|-------|
| `EvidenceKind`, `ReviewAction`, `FrameTerminalStatus`, `DecisionOutcome`, `DecisionJson` | Frozen (unchanged) |
| `BoundingBox` | Frozen: normalized `f32` `[0,1]` `x_min,y_min,x_max,y_max`; `PixelRect` derived ([ADR-0041](../adr/0041-normalized-bounding-box.md)) |
| `HsrpEvidence` | Frozen: `CueObservation` ternary cues (`ind_mark`, `hologram`, `geometry`) per ADR-0020 |
| `RectifiedPlate`, `OcrHypothesis`, `GrammarResult`, `OcrResult` | Frozen: required `DetectionId` |
| `GrammarStatus` | Frozen: `VALID` / `AMBIGUOUS` / `INVALID` (replaces `ok: bool`) |
| `MachineDecision` / `MachineOutcome` | Frozen struct: `ACCEPT` / `REVIEW_REQUIRED` / `REJECT_UNUSABLE` (was alias of `FusedDecision`) |
| `ReviewEvent`, `ReviewedResult`, `FinalStatus`, `AuditEvent` | Frozen (M09/M10 implement behavior) |
| `ExportRow` | Frozen (M11 serializes) |
| `Role`, `Permission` | Frozen: three operational roles ([ADR-0043](../adr/0043-operational-roles.md)) |
| `CostEvent`, `AiOperation` | Frozen (M03 emits, M10 persists) |
| IDs | `BatchId`, `JobId`, `FrameId`, `CameraId`, `TraceId`, `DetectionId`, `ConfigRevisionId`, `UserId`, `ReviewEventId`, `AuditEventId`, `ExportJobId` |
| `FusedDecision` | Stage 7 working state; projected via `MachineDecision::from_fused` |
| `PlateDetection` / `VehicleDetection` split | Deferred to M05 (additive only) |
| Storage ports / ArtifactState | Deferred to M10 (module-owned, not shared contracts) |

## Resolved D-items

| Item | Resolution |
|------|------------|
| HSRP `bool` vs ternary | Ternary `CueObservation`; booleans rejected on deserialize |
| BoundingBox f32 vs u32 | Normalized `f32` canon; `u32` only as derived `PixelRect` |

## Schema versions

- `INTAKE_SCHEMA_VERSION = "2.0"` (M02 envelope unchanged)
- `DECISION_SCHEMA_VERSION = "2.1"` (decision / export)

## Stage 6 naming (docs + Rust)

- Docs: **Observation Deduplication / Clustering** (not “Tracking”).
- Rust: `StageId::ObservationDedup`; display name `observation_dedup`.

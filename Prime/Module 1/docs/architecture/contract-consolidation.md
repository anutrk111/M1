# M01 Contract Consolidation

**HEAD baseline:** post-audit consolidation for shared types before M02–M12 implementation.  
**Rule:** classify A/B/C/D; only apply **B** in M01 Rust when safe without implementing later modules.

## Classification

| Contract | Class | Action |
|----------|-------|--------|
| `EvidenceKind`, `ReviewAction`, `FrameTerminalStatus`, `DecisionOutcome`, `DecisionJson` | **A** | Keep |
| `BoundingBox`, `Detections`, `RectifiedPlate`, `OcrHypothesis`, `GrammarResult`, `HsrpEvidence`, `FusedDecision`, `DedupResult` | **A** (skeleton) | Keep; extend where noted |
| `DetectionId` | **B** | Add newtype in `ids.rs` |
| `ProviderRef` | **B** | Add struct in `artifacts.rs` |
| `DetectionSummary` | **B** | Add struct |
| `ImageQualityGate` (`Accept`/`Degraded`/`Unusable`) | **B** | Add enum; optional on `ImageQualityReport` |
| `detection_id` on `Detection` / `RectifiedPlate` / `OcrHypothesis` | **B** | Optional fields (`serde` default) for lineage |
| `ConfigRevisionId` | **B** | Add newtype |
| `MachineDecision` | **B** | Type alias to `FusedDecision` + docs |
| `PlateDetection` / `VehicleDetection` split | **C** | Defer to M05 implementation |
| Full M06 OCR hypothesis / grammar statuses | **C** | Defer to M06 |
| `ReviewEvent` / `ReviewedResult` | **C** | Defer to M09 |
| Storage ports / ArtifactState | **C** | Defer to M10 |
| HSRP `bool` vs ternary observability | **D** | Keep fixture bools; M07 must map at implement time |
| BoundingBox f32 vs M05 u32 pixels | **D** | Normalize at M03/M05 boundary later; document |

## Stage 6 naming (docs + Rust)

- Docs: **Observation Deduplication / Clustering** (not “Tracking”).
- Rust: `StageId::ObservationDedup` (was `Dedup`); display name `observation_dedup`.
- `FrameContext.observation_dedup` replaces `.dedup` (same `DedupResult` payload type retained as observation-cluster skeleton).

## Compatibility

Optional fields default to `None` / empty so existing fixture JSON construction continues to compile. Fixture updated for renamed StageId/field.

## Out of scope here

Implementing M02–M12 crates, VAHAN, cameras, local GPU, UI.

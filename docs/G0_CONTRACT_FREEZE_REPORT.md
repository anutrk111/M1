# Gate G0 — Shared Contract Freeze Report

| Item | Value |
|------|-------|
| Base | `main` @ `f33b361acb8d24e61a0f8ef2083e711df31a89a4` |
| Branch | `impl/m01-contracts`, merged into `integration/m01-m03` |
| Governing ADRs | [0041](../Prime/Module%201/docs/adr/0041-normalized-bounding-box.md), [0042](../Prime/Module%201/docs/adr/0042-shared-contract-freeze.md), [0043](../Prime/Module%201/docs/adr/0043-operational-roles.md) |

## Decisions applied

| Decision | Implementation |
|----------|----------------|
| BoundingBox canon | Normalized `f32` `x_min,y_min,x_max,y_max` in `[0,1]`; validated in `new` and on deserialize. `PixelRect` derived via `to_pixel_rect`; `from_pixels` normalizes. |
| HSRP observability | `CueObservation { OBSERVED, NOT_OBSERVED, NOT_OBSERVABLE }` on `ind_mark`, `hologram`, `geometry`. Legacy booleans are rejected. |
| Lineage | `DetectionId` required on `RectifiedPlate`, `OcrHypothesis`, `GrammarResult`, `OcrResult`, `HsrpEvidence`, `MachineDecision`, `ReviewEvent`, `ReviewedResult`, `ExportRow`. |
| Roles | `Role { SUPER_ADMIN, CLERK, REVIEWING_OFFICER }` + `Permission` + `default_permissions()`. `auth.toml` is aligned. |
| Schema | `INTAKE_SCHEMA_VERSION = "2.0"` (unchanged), `DECISION_SCHEMA_VERSION = "2.1"`. |

## New shared contracts (`talos-types`)

- Grammar and OCR: `GrammarStatus`, `OcrResult` (with `validate_lineage`)
- Decision: `MachineOutcome` and `MachineDecision` (`from_fused`)
- Review: `ReviewEvent`, `FinalStatus`, `ReviewedResult`
- Audit and export: `AuditEvent`, `ExportRow`
- Access: `Role`, `Permission`
- AI accounting: `CostEvent`, `AiOperation`, `CostStatus`
- Geometry and errors: `PixelRect`, `ContractError`
- IDs: `UserId`, `ReviewEventId`, `AuditEventId`, `ExportJobId`

`talos-types` stays free of I/O and of any `talos-core` dependency. `talos-core` maps `ContractError` to `TalosError::Validation`.

## Code touched outside talos-types

- `talos-core/src/fixture.rs`:
  - normalized bboxes and ternary HSRP;
  - lineage is now required, and a missing `DetectionId` returns `Validation`;
  - `validate_indian_plate(detection_id, raw)` returns `GrammarStatus`.
- `talos-core/src/pipeline.rs`: `DecisionJson.schema_version` now uses `DECISION_SCHEMA_VERSION`.
- `talos-core/src/error.rs`: adds `From<ContractError>`.

## Tests

Golden fixtures live in `Prime/Module 1/crates/talos-types/tests/fixtures/`, exercised by `tests/contracts.rs` (18 tests):

- round-trips: bbox, HSRP, OCR result, machine decision, review event, export row, cost event
- legacy rejection: boolean HSRP, rectified plate without lineage, pixel bbox, missing `detection_id`
- bbox invariants, pixel conversion and resolution independence
- frozen ternary serde names, OCR lineage mismatch, conservative outcome mapping
- `SuperAdmin` has no review authority; schema versions

## Verification (from `Prime/`)

| Command | Exit |
|---------|------|
| `cargo metadata --no-deps` | 0 |
| `cargo fmt --all -- --check` | 0 |
| `cargo check --workspace --all-targets --all-features` | 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 |
| `cargo test --workspace --all-features` | 0 (62 passed, 0 failed) |
| `git diff --check` | 0 |

## Docs aligned

- Updated:
  - `Prime/ADRs.md` (0041–0043)
  - `contract-consolidation.md`
  - `REPOSITORY_CONTRACT_AUDIT.md` (D-items resolved)
  - M05 PDR, wiki and overview (normalized bbox)
  - M04 PDR (bbox to `PixelRect` at crop)
  - M12 PDR and wiki, `auth.toml`
  - M09 `review.toml` role mapping
- Status notes: ADR-0011 item 2 superseded; ADR-0034 and ADR-0036 annotated.

## CI

`.github/workflows/ci.yml` triggers on `main`, `integration/**`, `impl/**`, `release/**` (push) and PRs into `main` / `integration/**` / `release/**`. Adds a `cargo metadata --no-deps` step.

## Result

**Gate G0: GREEN.**

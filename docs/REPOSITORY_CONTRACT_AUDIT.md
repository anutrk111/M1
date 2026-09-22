# Repository Contract Audit

**Repo:** Talos-RS (`anutrk111/M1`)  
**Audit HEAD:** `07fae6d4c5ce67e8e710f849785bd5c9dce0b30e` (`main`)  
**Scope:** Docs-first M01–M12 spine + implemented M01 Rust workspace  
**Date:** 2026-09-22

Evidence-based. Paths are relative to repository root unless noted.

---

## 1. Module status

| Module | Path | Status | Evidence |
|--------|------|--------|----------|
| M01 | `Prime/Module 1/` | **Implemented** (types, config, pipeline fixture, health apps) | `Prime/Module 1/Cargo.toml` workspace; crates under `crates/` |
| M02 | `Prime/Module 2/` | Docs-only PDR | `docs/pdr/M02-batch-and-file-intake.md`; no Rust crate |
| M03 | `Prime/Module 3/` | Docs-only PDR | `docs/pdr/M03-ai-provider-gateway.md` |
| M04 | `Prime/Module 4/` | Docs-only PDR | `docs/pdr/M04-image-pre-processing.md` |
| M05 | `Prime/Module 5/` | Docs-only PDR | `docs/pdr/M05-vision-processing.md` |
| M06 | `Prime/Module 6/` | Docs-only PDR | `docs/pdr/M06-ocr-plate-intelligence.md` |
| M07 | `Prime/Module 7/` | Docs-only PDR | `docs/pdr/M07-hsrp-analysis.md` |
| M08 | `Prime/Module 8/` | Docs-only PDR | `docs/pdr/M08-confidence-decision.md` |
| M09 | `Prime/Module 9/` | Docs-only PDR | `docs/pdr/M09-review-management.md` |
| M10 | `Prime/Module 10/` | Docs-only PDR | `docs/pdr/M10-storage-and-data.md` |
| M11 | `Prime/Module 11/` | Docs-only PDR | `docs/pdr/M11-export-and-reporting.md` |
| M12 | `Prime/Module 12/` | Docs-only PDR | `docs/pdr/M12-auth-admin-config.md` |

Index: [`Prime/README.md`](../Prime/README.md).

---

## 2. Implemented vs docs-only (Rust)

**Implemented workspace** (`Prime/Module 1/Cargo.toml` members):

- `talos-types`, `talos-config`, `talos-core`, `observability`
- Apps: `talos-api`, `talos-worker`, `review-api`, `admin-gateway` (health spines)

**Not implemented:** any `talos-ocr`, `talos-hsrp`, `talos-vision`, `talos-storage`, etc. under Modules 2–12.

---

## 3. Contract mismatches (M01 Rust vs later PDRs)

Classification per Phase 4 (detail in [`Prime/Module 1/docs/architecture/contract-consolidation.md`](../Prime/Module%201/docs/architecture/contract-consolidation.md)):

| Contract | PDR source | M01 Rust (`talos-types`) | Class |
|----------|------------|-------------------------|-------|
| `DetectionId` | M05/M06/M07 | **Missing** | B (add shared newtype) |
| `BoundingBox` | M05 | Present (`x,y,w,h` f32) — M05 PDR uses `u32` pixel fields | D (doc vs code field types) |
| `ProviderRef` | M05 | **Missing** | B |
| `PlateDetection` / `VehicleDetection` | M05 | Generic `Detection` only | C (evolve when M05 builds) |
| `DetectionSummary` | M05 | **Missing** | B |
| Quality `Accept`/`Degraded`/`Unusable` | M04 | Only `ImageQualityReport` scores | B (add gate enum) |
| `RectifiedPlate` | M04 | Present; **no `DetectionId`** | B (optional field) |
| `OcrHypothesis` | M06 structured | Present as text+confidence only | C (full structure at M06) |
| `GrammarResult` statuses | M06 VALID/… | `ok: bool` only | C |
| HSRP ternary observability | M07 | `bool` flags on `HsrpEvidence` | D (bool vs Observed/NotObservable) |
| `MachineDecision` | M09/M11 | Closest: `FusedDecision` / `DecisionJson` | B (alias + docs) |
| `ReviewEvent` / `ReviewedResult` | M09 | **Missing** | C |
| `ConfigRevision` id | M12/M08 | **Missing** | B (id newtype only) |
| `ReviewAction`, `FrameTerminalStatus`, `EvidenceKind` | M01/M09 | Present | A |

---

## 4. Stale / conflicting terminology

| Issue | Where | Problem |
|-------|-------|---------|
| Stage 6 name | `Prime/Module 1/docs/architecture/overview.md` L69; M01 PDR L292 | Still **“Deduplication & Tracking”** (implies tracking) |
| Stage 6 name | `Prime/Module 1/wiki/Pipeline-Stages.md` L13 | Already **“Observation Dedup (clusters)”** |
| M05 config section | `Prime/Module 5/configs/detection.toml` `[dedup]` | Behavior is **detection suppression**, not Stage 6 dedup |
| ADR-0011 L17 | References `[dedup]` IoU as suppression | Correct concept; wrong config key name |
| M05 PDR §8 | “Pipeline deduplication” for Stage 6 | Prefer **observation deduplication / clustering** (M08 ADR-0022) |

---

## 5. Duplicate / conflicting definitions

| Topic | Conflict |
|-------|----------|
| `decision.toml` | M01 has 4 keys; M08 extends with `require_eligible_for_primary`, `[decision.dedup]`, `[decision.vlm]` — intentional module-local extension; M01 loader does **not** read M08 file |
| ADR wiki copies | M01 wiki mirrors ADR-0001..0003 (not duplicate numbers in `docs/adr/`) |
| ADR sequence by ownership | Numbers 0019–0020 appear after 0016–0018 by module ownership (M06/M07 after M08 batch) — **not renumbered**; flagged in registry |
| HSRP booleans vs ternary | M01 `HsrpEvidence` vs M07 ADR-0020 |

---

## 6. Missing shared M01 types (summary)

Needed as shared foundations before M02–M05 code: `DetectionId`, `ProviderRef`, `DetectionSummary`, `ImageQualityGate`, optional `detection_id` on plate artifacts, `ConfigRevisionId`, `MachineDecision` alias. Full list in contract-consolidation.md.

---

## 7. Relative links

Spot-check M05–M08 architecture overviews → Module 7/8: **resolve OK** at this HEAD.  
Root `README.md` previously only pointed at Module 1 (improved in this baseline work).

No automated link crawler existed before this baseline (CI adds lightweight checks).

---

## 8. Config naming conflicts

| Config | Owners | Notes |
|--------|--------|-------|
| `decision.toml` | M01 runtime + M08 module mirror | Shared threshold keys align; M08-only keys not loaded by `talos-config` today |
| `[dedup]` under M05 | Suppression | Rename to `[suppression]` (no Rust consumer) |
| `[decision.dedup]` under M08 | Observation clustering | Different namespace — keep; document distinction |

---

## 9. ADR numbering / ownership

Canonical ADRs: **0001–0040**, one file each under module `docs/adr/`. **No number collisions** in `docs/adr/`.  
Global index: [`Prime/ADRs.md`](../Prime/ADRs.md).

---

## 10. CI gaps (pre-baseline)

| Gap | Evidence |
|-----|----------|
| No `.github/workflows/` | Absent at audit HEAD |
| No root `docs/` | Absent at audit HEAD |
| No module-tree validation | Manual only |

---

## 11. Implementation blockers (for M02+)

1. Shared lineage IDs (`DetectionId`) not in M01 types (addressed as safe extension in this baseline).
2. Quality gate enum missing for Stage 0 halt semantics (safe extension).
3. HSRP bool vs ternary — **architecture decision** before implementing M07 types in M01.
4. BoundingBox f32 vs u32 pixel canon — decide at M05 implementation boundary.
5. GitHub Wiki remote not bootstrapped (`M1.wiki.git` 404 until first UI page).
6. Do not implement M02–M12 business logic until contracts green and CI baseline green.

---

## 12. Locked product posture (unchanged)

Batch/file only; no cameras; no VAHAN; no local GPU default; AI via M03; immutable originals; no fake success; explicit `NotImplemented`; DetectionId lineage; secrets never committed.

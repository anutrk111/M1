# Baseline Readiness Report

**Product:** Talos-RS — Automated Vehicle Identification & HSRP Audit Engine  
**Task:** Audit → contract consolidation → doc cleanup → CI baseline (no M02–M12 business logic)

## 1. Branch

`main`

## 2. HEAD SHA before work

`07fae6d4c5ce67e8e710f849785bd5c9dce0b30e` (equal to `origin/main` at start; not diverged)

## 3. HEAD SHA after work

Recorded at commit time in git log (this baseline commit on `main` after verification).

## 4. Files changed (summary)

| Area | Change |
|------|--------|
| `docs/REPOSITORY_CONTRACT_AUDIT.md` | Phase 1 audit |
| `docs/BASELINE_READINESS_REPORT.md` | This report |
| `Prime/ADRs.md` | Global ADR registry 0001–0040 |
| `Prime/Module 1/docs/architecture/contract-consolidation.md` | A/B/C/D classification |
| `Prime/Module 1/crates/talos-types` | DetectionId, ConfigRevisionId, ProviderRef, ImageQualityGate, DetectionSummary, lineage optional fields, MachineDecision alias |
| `Prime/Module 1/crates/talos-core` | StageId::ObservationDedup, FrameContext.observation_dedup, fixture lineage |
| M01/M05 docs + wiki | Stage 6 naming; `[suppression]` terminology |
| `Prime/Module 5/configs/detection.toml` | `[dedup]` → `[suppression]` (no Rust consumer) |
| Root `README.md`, `Prime/README.md` | Architecture posture + links |
| `.github/workflows/ci.yml` | Rust + docs-contracts CI |
| Clippy/fmt touch-ups | validate.rs, fixture indexing, overview link depth |

## 5. Contract inconsistencies found

- Stage 6 labeled “Deduplication & Tracking” in M01 PDR/overview vs observation clustering in M08
- M05 `[dedup]` named like Stage 6 but means detection suppression
- Missing shared types: DetectionId, ProviderRef, quality gates, DetectionSummary, ConfigRevisionId
- HSRP bool fields vs M07 ternary (flagged **D**)
- BoundingBox f32 vs M05 u32 pixel canon (flagged **D**)
- M01 vs M08 `decision.toml` extension keys (documented; M01 loader only reads M01 configs)
- No CI / root docs at pre-baseline HEAD
- Overview PDR relative links used wrong `../` depth (`../../` vs `../../../`)

## 6. Contract inconsistencies fixed

- Stage 6 docs → **Observation Deduplication / Clustering**
- M05 config → `[suppression]`; ADR-0011 / PDR / wiki aligned
- Shared M01 type extensions (**B**) with serde-optional backward-compatible fields
- `MachineDecision` = type alias of `FusedDecision`
- Global ADR registry + audit + consolidation docs
- Root README posture; CI workflow; overview link depth

## 7. Remaining blockers

- **D:** HSRP ternary vs bool — resolve when implementing M07 types
- **D:** BoundingBox coordinate type — normalize at M05/M03 boundary
- **C:** Full OCR/grammar/review/storage structs — deferred to owning modules
- GitHub Wiki `M1.wiki.git` still needs first UI page before `publish-wiki.sh`
- M02–M12 Rust implementation intentionally not started

## 8–11. Cargo results (local, evidence)

Working directory: `Prime/Module 1`

| Command | Exit |
|---------|------|
| `cargo fmt --all -- --check` | **0** (after `cargo fmt --all`) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **0** |
| `cargo test --workspace --all-features` | **0** (13 tests) |
| `cargo check --workspace --all-targets --all-features` | **0** |

## 12. Documentation / link validation

| Check | Exit |
|-------|------|
| Modules 1–12 dirs + required PDRs + ADRs.md + audit | **0** |
| Duplicate ADR numbers under `*/docs/adr/` | **0** (40 unique) |
| overview.md relative link spot-check | **0** |

## 13. GitHub Actions workflow

Added: [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)  
Jobs: `rust` (fmt/clippy/test/check in `Prime/Module 1`), `docs-contracts` (tree/PDR/ADR/link checks).

## 14. Ready for M02 implementation?

**Shared M01 contracts:** sufficient to begin M02 **design-aligned** implementation of intake against M01 types/errors — **yes**, with noted **D** items deferred.

**M02–M12 business logic:** not implemented in this task (by design).

## Final state

**GREEN WITH DOCUMENTED DEFERRED ITEMS**

Mandatory implemented-workspace checks (fmt, clippy, test, check) and docs-contracts checks passed. Deferred items are architecture decisions (**D**) and future-module types (**C**), plus wiki remote bootstrap.

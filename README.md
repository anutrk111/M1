# Talos-RS

**Automated Vehicle Identification & HSRP Audit Engine** (100% Rust / Tokio hexagonal modular monorepo).

## Current architecture

| Layer | Status |
|-------|--------|
| **M01 Foundation & Core** | Implemented / scaffolded (types, config, pipeline fixture, health apps) |
| **M02–M12** | **PDR / design contracts** (docs-only unless a module later gains Rust) |

## Locked deployment posture

- **API-first**, **batch/file intake** (folders, ZIP, uploads)
- **No** direct ANPR camera integration
- **No** VAHAN API in this posture
- Vision / OCR / HSRP via **external AI providers through M03** initially
- **CPU-first** local runtime; future local GPU only via explicit abstraction (`NotImplemented` until enabled)
- **Immutable** original images; **no fake success**; missing backends fail explicitly
- Lineage: `OriginalFrame → DetectionId → RectifiedPlate → OCR/HSRP → MachineDecision → Review/Export`
- **Secrets never committed**

This is **not** a claim of production accuracy or full-system readiness.

## Module index (M01–M12)

See the authoritative table in **[Prime/README.md](Prime/README.md)**.

## Key docs

| Doc | Purpose |
|-----|---------|
| [Prime/README.md](Prime/README.md) | Module paths and PDR status |
| [M01 architecture](Prime/Module%201/docs/architecture/overview.md) | Hex map + stages |
| [Contract consolidation](Prime/Module%201/docs/architecture/contract-consolidation.md) | Shared-type classification A/B/C/D |
| [Global ADR registry](Prime/ADRs.md) | ADR 0001–0040 index |
| [Repository contract audit](docs/REPOSITORY_CONTRACT_AUDIT.md) | Audit findings |
| [Baseline readiness report](docs/BASELINE_READINESS_REPORT.md) | CI/verification evidence |

## Develop (M01 workspace)

```bash
cd "Prime/Module 1"
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

CI: [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

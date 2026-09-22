# Talos-RS Architecture Overview

Talos-RS is an **Automated Vehicle Identification & HSRP Audit Engine** built as a **100% Rust** hexagonal modular monorepo on the **Tokio** async runtime.

## Product posture (current generation)

| Dimension | Choice |
|-----------|--------|
| Interface | API-first |
| Intake | Batch / file (folders, ZIP, CSV/XLSX metadata) — **not** live camera streams |
| Vision / OCR / HSRP | External **AI Provider Gateway** (Vision + VLM APIs) |
| Local GPU | Extension only (`NotImplemented` until enabled) |
| VAHAN / transport DB | Out of scope (types reserved; no adapters) |
| Scale target | Batch workloads sized from small pilots up to multi-camera historical data (e.g. up to 178 camera *data* sources as files), not live 178-camera ingest |

## Hexagonal module map

```text
                    M02 Batch & File Intake
                              │
        M12 Auth/Admin ───────┼─────── M03 AI Provider Gateway
                              │
        M11 Export ───────────┼─────── M04 Image Pre-Processing
                              │
                         ┌────┴────┐
                         │  M01    │
                         │Foundation│
                         │ & Core  │
                         └────┬────┘
                              │
        M10 Storage ──────────┼─────── M05 Vision (via AI API)
                              │
        M09 Review ───────────┼─────── M06 OCR & Plate Intelligence
                              │
        M08 Confidence/Decision ┼───── M07 HSRP Analysis (via AI API)
```

M01 is the **central dependency hub**: shared types, config, errors, Stage 0–7 pipeline framework, tracing, health, and runtime lifecycle. Other modules depend on M01 contracts; they do not fork parallel schemas.

## Module index

| ID | Module | Role | PDR |
|----|--------|------|-----|
| **M01** | Foundation & Core | Types, config, errors, pipeline Stages 0–7, tracing, health, lifecycle | [M01-foundation-and-core.md](../pdr/M01-foundation-and-core.md) |
| M02 | Batch & File Intake | Folder / ZIP / upload intake, metadata extraction | TBD |
| M03 | AI Provider Gateway | Vision / VLM API integration, fallbacks, cost tracking | TBD |
| M04 | Image Pre-Processing | Quality checks, enhancement, cropping for AI APIs | TBD |
| M05 | Vision Processing | Vehicle and plate detection (via M03) | TBD |
| M06 | OCR & Plate Intelligence | OCR hypotheses, Indian registration grammar, evidence-based correction | TBD |
| M07 | HSRP Analysis | IND region, hologram, geometry, color/layout cues (via M03) | TBD |
| M08 | Confidence & Decision | Multi-factor fusion; Auto vs Secondary vs Review thresholds | TBD |
| M09 | Review Management | Human VERIFY / CORRECT / REJECT / UNREADABLE | TBD |
| M10 | Storage & Data | PostgreSQL, TimescaleDB, MinIO/S3 | TBD |
| M11 | Export & Reporting | CSV/XLSX exports, summary statistics | TBD |
| M12 | Auth, Admin & Config | Users, RBAC, MFA, system settings | TBD |

## Pipeline (Stages 0–7)

Orchestrated by **M01**; implemented by M04–M08 (remote vision via M03):

| Stage | Name |
|-------|------|
| 0 | Image Quality Assessment |
| 1 | Vehicle + Plate Detection |
| 2 | Plate Rectification & Enhancement |
| 3 | OCR |
| 4 | Indian Registration Grammar |
| 5 | HSRP Evidence |
| 6 | Deduplication & Tracking |
| 7 | Confidence Fusion & Decision |

Normative contracts, decision thresholds, and evidence separation rules are defined in the [M01 PDR](../pdr/M01-foundation-and-core.md).

## Evidence model

- **`VisualObservation`** — produced by the visual / VLM pipeline.
- **`ExternalDatabaseVerification`** — reserved for authorized future integrations (e.g. VAHAN). Visual stages must never write this kind.

VLM may add evidence for ambiguous cases; it must not silently override deterministic grammar or configured hard-fail rules.

## Decision thresholds (config-driven)

Defaults (override in `configs/decision.toml`):

| Outcome | Default band |
|---------|----------------|
| Auto-approved | ≥ 0.90 |
| Secondary verification | 0.80 – 0.90 |
| Review required | < 0.80 |

## Inputs and outputs

**Inputs:** JPG/PNG folders, ZIP uploads, optional CSV/XLSX metadata (camera ID, timestamp, location).

**Outputs:** Decision JSON, CSV/XLSX exports (M11), audit logs, processed / evidence images (M10).

**Users:** Transport administration, review officers (web UI), system administrators, monitoring / operations.

## Implementation policy

- Scaffold module boundaries early; do **not** fake success paths for missing backends.
- Hardware / provider-dependent code returns **`NotImplemented`** until real credentials, models, or services exist.
- First spine: fixture-backed `image → Stages 0–7 → decision JSON` offline.

## Related ADRs (to be authored at implementation time)

- `docs/adr/0001-pipeline-stage-contract.md`
- `docs/adr/0002-evidence-kind-separation.md`
- `docs/adr/0003-no-local-gpu-default.md`

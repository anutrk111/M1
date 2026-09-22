# Architecture

Talos-RS is a **hexagonal modular monorepo**. **M01 Foundation & Core** is the central dependency hub.

## Module map

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

## Module index

| ID | Module | Role |
|----|--------|------|
| **M01** | Foundation & Core | Types, config, errors, Stages 0–7, tracing, health, lifecycle |
| M02 | Batch & File Intake | Folder / ZIP / upload intake |
| M03 | AI Provider Gateway | Vision / VLM APIs, cost tracking |
| M04 | Image Pre-Processing | Quality, enhance, crop |
| M05 | Vision Processing | Vehicle / plate detection |
| M06 | OCR & Plate Intelligence | OCR + Indian registration grammar |
| M07 | HSRP Analysis | IND / hologram / geometry cues |
| M08 | Confidence & Decision | Fusion + thresholds |
| M09 | Review Management | VERIFY / CORRECT / REJECT / UNREADABLE |
| M10 | Storage & Data | Postgres / Timescale / MinIO |
| M11 | Export & Reporting | CSV / XLSX |
| M12 | Auth, Admin & Config | RBAC / MFA / settings |

## Evidence model

- `VisualObservation` — vision, OCR, HSRP, VLM
- `ExternalDatabaseVerification` — reserved (e.g. future VAHAN); visual stages must never write this

See [[M01-Foundation-and-Core]] and [[ADRs]].

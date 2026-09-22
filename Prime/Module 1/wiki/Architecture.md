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
| M02 | Batch & File Intake | Folder / ZIP / upload intake — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%202/docs/pdr/M02-batch-and-file-intake.md) |
| M03 | AI Provider Gateway | Vision / VLM APIs, cost tracking — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%203/docs/pdr/M03-ai-provider-gateway.md) |
| M04 | Image Pre-Processing | Stage 0/2 quality + multi-plate rectify — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%204/docs/pdr/M04-image-pre-processing.md) |
| M05 | Vision Processing | Stage 1 detect via M03 — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%205/docs/pdr/M05-vision-processing.md) |
| M06 | OCR & Plate Intelligence | OCR + grammar — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%206/docs/pdr/M06-ocr-plate-intelligence.md) |
| M07 | HSRP Analysis | HSRP observations — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%207/docs/pdr/M07-hsrp-analysis.md) |
| M08 | Confidence & Decision | Fusion + thresholds — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%208/docs/pdr/M08-confidence-decision.md) |
| M09 | Review Management | Review workflow — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%209/docs/pdr/M09-review-management.md) |
| M10 | Storage & Data | Persist via ports — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%2010/docs/pdr/M10-storage-and-data.md) |
| M11 | Export & Reporting | Export views — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%2011/docs/pdr/M11-export-and-reporting.md) |
| M12 | Auth, Admin & Config | AuthZ / admin — [PDR](https://github.com/anutrk111/M1/blob/main/Prime/Module%2012/docs/pdr/M12-auth-admin-config.md) |

## Evidence model

- `VisualObservation` — vision, OCR, HSRP, VLM
- `ExternalDatabaseVerification` — reserved (e.g. future VAHAN); visual stages must never write this

See [[M01-Foundation-and-Core]] and [[ADRs]].

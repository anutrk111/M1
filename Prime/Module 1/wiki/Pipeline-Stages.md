# Pipeline Stages (0–7)

Orchestrated by **M01** (`talos-core::Pipeline`). Stage logic lives in M04–M08 (remote vision via M03).

| Stage | Name | Primary module |
|-------|------|----------------|
| 0 | Image Quality Assessment | M04 |
| 1 | Vehicle + Plate Detection | M05 via M03 |
| 2 | Plate Rectification & Enhancement | M04 |
| 3 | OCR | [M06](../../Module%206/docs/pdr/M06-ocr-plate-intelligence.md) via M03 |
| 4 | Indian Registration Grammar | [M06](../../Module%206/docs/pdr/M06-ocr-plate-intelligence.md) |
| 5 | HSRP Evidence | [M07](../../Module%207/docs/pdr/M07-hsrp-analysis.md) via M03 |
| 6 | Observation Deduplication / Clustering | [M08](../../Module%208/docs/pdr/M08-confidence-decision.md) / M10 |
| 7 | Confidence Fusion & Decision | [M08](../../Module%208/docs/pdr/M08-confidence-decision.md) |

## Stage contract

```rust
async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError>;

enum StageStatus { Continue, SkipRemaining, Halt }
```

- **Continue** — next stage  
- **SkipRemaining** — stop; do not invent OCR/detections  
- **Halt** — set `FrameTerminalStatus::Halted`  
- **NotImplemented** — set `FailedNotImplemented`; never fabricate plate text  

## Backends (`configs/pipeline.toml`)

| Backend | Meaning |
|---------|---------|
| `fixture` | Deterministic CI / local spine |
| `ai_api` | External providers via M03 (production path) |
| `local_gpu_ext` | ONNX/TensorRT — `NotImplemented` until enabled |

## Decision thresholds (`configs/decision.toml`)

| Outcome | Starting default band |
|---------|------------------------|
| AutoApproved | ≥ 0.90 |
| SecondaryVerification | 0.80 – 0.90 |
| ReviewRequired | < 0.80 |

These are **starting configuration defaults only**, not validated production accuracy thresholds. Normative fusion: [M08 PDR](../../Module%208/docs/pdr/M08-confidence-decision.md).

Full detail: [[M01-Foundation-and-Core]] · ADR: [[ADRs]]

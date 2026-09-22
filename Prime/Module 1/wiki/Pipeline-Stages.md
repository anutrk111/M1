# Pipeline Stages (0–7)

Orchestrated by **M01** (`talos-core::Pipeline`). Stage logic lives in M04–M08 (remote vision via M03).

| Stage | Name | Primary module |
|-------|------|----------------|
| 0 | Image Quality Assessment | M04 |
| 1 | Vehicle + Plate Detection | M05 via M03 |
| 2 | Plate Rectification & Enhancement | M04 |
| 3 | OCR | M06 via M03 |
| 4 | Indian Registration Grammar | M06 (deterministic preferred) |
| 5 | HSRP Evidence | M07 via M03 |
| 6 | Deduplication & Tracking | M08 / M10 |
| 7 | Confidence Fusion & Decision | M08 |

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

| Outcome | Default |
|---------|---------|
| AutoApproved | ≥ 0.90 |
| SecondaryVerification | 0.80 – 0.90 |
| ReviewRequired | < 0.80 |

Thresholds are config-driven, not hardcoded.

Full detail: [[M01-Foundation-and-Core]] · ADR: [[ADRs]]

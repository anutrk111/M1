# Module 4 — Image Pre-Processing (M04)

Stage 0 quality evidence and Stage 2 multi-plate rectification/enhancement. Prepares pixels; does not assign meaning.

## Status

**PDR complete** (docs only). Rust `talos-preprocess` not started.

## Contents

| Path | Purpose |
|------|---------|
| [docs/pdr/M04-image-pre-processing.md](docs/pdr/M04-image-pre-processing.md) | Normative PDR (incl. M01 type extensions) |
| [docs/adr/0008-stage0-quality-gates.md](docs/adr/0008-stage0-quality-gates.md) | Accept / Degraded / Unusable |
| [docs/adr/0009-rectify-after-detection.md](docs/adr/0009-rectify-after-detection.md) | Multi-plate + provenance + NoPlateDetected |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Overview |
| [configs/preprocess.toml](configs/preprocess.toml) | Starting defaults |
| [wiki/](wiki/) | In-module wiki |

## Depends on

[Module 1](../Module%201/) · [Module 2](../Module%202/) (originals) · [Module 3](../Module%203/) (downstream plate ops)

# Module 5 — Vision Processing (M05)

Stage 1 vehicle + plate detection on the original frame via M03. Talos-owned DetectionId lineage into M04/M06/M07/M08.

## Status

**PDR complete** (docs only). Rust `talos-vision` not started.

## Contents

| Path | Purpose |
|------|---------|
| [docs/pdr/M05-vision-processing.md](docs/pdr/M05-vision-processing.md) | Normative PDR |
| [docs/adr/0010-detection-via-m03-only.md](docs/adr/0010-detection-via-m03-only.md) | M03-only egress |
| [docs/adr/0011-multi-plate-detection-ids.md](docs/adr/0011-multi-plate-detection-ids.md) | IDs, bbox, association, suppression |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Overview |
| [configs/detection.toml](configs/detection.toml) | Starting defaults |
| [wiki/](wiki/) | In-module wiki |

## Depends on

[Module 1](../Module%201/) · [Module 3](../Module%203/) · [Module 4](../Module%204/)

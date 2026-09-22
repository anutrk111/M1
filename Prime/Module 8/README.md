# Module 8 — Confidence & Decision (M08)

Stages 6–7: observation deduplication (≠ M05 suppression) and evidence fusion with **eligibility then rank**. Full candidate provenance retained. VLM additive only.

## Status

**PDR complete** (docs only). Rust `talos-decision` not started.

## Contents

| Path | Purpose |
|------|---------|
| [docs/pdr/M08-confidence-decision.md](docs/pdr/M08-confidence-decision.md) | Normative PDR |
| [docs/adr/0016-config-driven-thresholds.md](docs/adr/0016-config-driven-thresholds.md) | 0.90/0.80 starting defaults |
| [docs/adr/0017-vlm-selective-additive.md](docs/adr/0017-vlm-selective-additive.md) | VLM cannot erase hard evidence |
| [docs/adr/0018-no-plate-vs-hard-fail.md](docs/adr/0018-no-plate-vs-hard-fail.md) | NoPlateDetected policy |
| [docs/adr/0021-eligibility-then-rank.md](docs/adr/0021-eligibility-then-rank.md) | Hard constraints before ranking |
| [docs/adr/0022-observation-cluster-not-vehicle.md](docs/adr/0022-observation-cluster-not-vehicle.md) | Stage 6 cluster ≠ same vehicle |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Overview |
| [configs/decision.toml](configs/decision.toml) | Starting defaults |
| [wiki/](wiki/) | In-module wiki |

## Depends on

[Module 1](../Module%201/) · [Module 3](../Module%203/) · [Module 4](../Module%204/) · [Module 5](../Module%205/) · [Module 6](../Module%206/) · [Module 7](../Module%207/)

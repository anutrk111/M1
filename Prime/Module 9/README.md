# Module 9 — Review Management (M09)

Human review officer workflow over machine decisions. MachineDecision stays immutable; ReviewEvent is append-only; ReviewedResult is the effective human-reviewed state.

## Status

**PDR complete** (docs only). Rust review routes / UI not started. `review-api` remains M01 health spine until M09 build.

## Contents

| Path | Purpose |
|------|---------|
| [docs/pdr/M09-review-management.md](docs/pdr/M09-review-management.md) | Normative PDR |
| [docs/adr/0023-review-actions-m01-only.md](docs/adr/0023-review-actions-m01-only.md) | M01 actions; workflow ≠ FrameTerminalStatus |
| [docs/adr/0024-review-immutable-machine-decision.md](docs/adr/0024-review-immutable-machine-decision.md) | Three-layer audit model |
| [docs/adr/0025-correct-requires-detection-id.md](docs/adr/0025-correct-requires-detection-id.md) | Correct field contract |
| [docs/adr/0029-review-claim-lease.md](docs/adr/0029-review-claim-lease.md) | Claim / lease concurrency |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Overview |
| [configs/review.toml](configs/review.toml) | Starting defaults |
| [wiki/](wiki/) | In-module wiki |

## Depends on

[Module 1](../Module%201/) · [Module 8](../Module%208/) · [Module 10](../Module%2010/) (persist via ports)

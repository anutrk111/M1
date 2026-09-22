# Getting Started

## Prerequisites

- Rust 1.75+ (`rustc`, `cargo`)
- macOS / Linux

## Clone and test

```bash
git clone https://github.com/anutrk111/M1.git
cd M1
cargo test
```

## Run the M01 fixture spine

Offline path (no GPU, no network):

```bash
cargo run -p talos-worker -- run-fixture --config configs --out decision.json
```

Produces schema v2.0 decision JSON with Stages 0–7 timings, visual evidence, and an outcome (`AutoApproved` | `SecondaryVerification` | `ReviewRequired`).

## Crates (M01)

| Crate | Role |
|-------|------|
| `talos-types` | IDs, intake envelope, evidence, decision schemas |
| `talos-config` | Layered config (TOML + env), fail-closed validation |
| `talos-core` | Pipeline, errors, runtime, health, fixture stages |
| `observability` | Metric / log field name conventions |

## Apps

| Binary | Port (default) | Notes |
|--------|----------------|-------|
| `talos-worker` | — / 8080 health | `run-fixture`, `serve-health` |
| `talos-api` | 8081 | Health spine; intake in M02 |
| `review-api` | 8082 | Health spine; review routes in [M09](../../Module%209/docs/pdr/M09-review-management.md) |
| `admin-gateway` | 8083 | Health spine; admin/auth in [M12](../../Module%2012/docs/pdr/M12-auth-admin-config.md) |

Health contract: `GET /healthz`, `GET /readyz`.

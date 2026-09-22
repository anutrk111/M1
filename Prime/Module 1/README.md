# Talos-RS

Automated Vehicle Identification & HSRP Audit Engine (Rust / Tokio).

## Current status

**M01 Foundation & Core** is implemented:

| Crate | Role |
|-------|------|
| `talos-types` | Shared IDs, intake envelope, evidence, decision schemas |
| `talos-config` | TOML + `TALOS__*` env, fail-closed validation |
| `talos-core` | Pipeline Stages 0–7, errors, runtime, health, fixture spine |
| `observability` | Metric / log field name conventions |

Apps (`talos-api`, `talos-worker`, `review-api`, `admin-gateway`) depend on M01 and expose `/healthz` + `/readyz`.

## Docs & Wiki

- **GitHub Wiki:** https://github.com/anutrk111/M1/wiki
- In-repo mirror: [`wiki/`](wiki/)
- [Architecture](docs/architecture/overview.md) · [M01 PDR](docs/pdr/M01-foundation-and-core.md) · [ADRs](docs/adr/)

## Quick start

Canonical workspace root is **`Prime/`** (not this module):

```bash
cd Prime
cargo test --workspace --all-features
cargo run -p talos-worker -- run-fixture --config "Module 1/configs" --out decision.json
```

See [`CARGO_WORKSPACE.md`](CARGO_WORKSPACE.md).

## Config

See `configs/*.toml`. Override with env, e.g. `TALOS__DECISION__AUTO_APPROVE_MIN=0.95`.

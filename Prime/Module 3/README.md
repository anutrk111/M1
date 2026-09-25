# Module 3 — AI Provider Gateway (M03)

Sole egress for external Vision + VLM APIs: clients, fallbacks, cost tracking, additive VLM evidence.

## Status

**Implemented** as `crates/talos-ai-gateway`, the adapter for the M01 port `talos_core::VisionGateway`. It provides:

- fixture and `http_json` providers
- timeout, bounded retry with jitter, and ordered failover
- per-provider circuit breaker and token-bucket rate limit
- OCR tie-breaker ("Emperor")
- `CostEvent` accounting, metrics snapshot, provider health

See [docs/M03_IMPLEMENTATION_REPORT.md](docs/M03_IMPLEMENTATION_REPORT.md).

```bash
cd Prime
cargo test -p talos-ai-gateway
```

## Contents

| Path | Purpose |
|------|---------|
| [crates/talos-ai-gateway/](crates/talos-ai-gateway/) | Gateway crate |
| [docs/M03_IMPLEMENTATION_REPORT.md](docs/M03_IMPLEMENTATION_REPORT.md) | Implementation report (policy, wire format, tests) |
| [docs/pdr/M03-ai-provider-gateway.md](docs/pdr/M03-ai-provider-gateway.md) | Normative PDR |
| [docs/adr/0006-vlm-evidence-additive-only.md](docs/adr/0006-vlm-evidence-additive-only.md) | VLM evidence rules |
| [docs/adr/0007-provider-error-mapping.md](docs/adr/0007-provider-error-mapping.md) | HTTP → TalosError |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Module overview |
| [configs/ai_providers.toml](configs/ai_providers.toml) | Defaults (no secrets) |
| [wiki/](wiki/) | In-module wiki |

## Depends on

[Module 1 — Foundation & Core](../Module%201/) (`talos-types` frozen contracts, `talos-core` port + `TalosError`).

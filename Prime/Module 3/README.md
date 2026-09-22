# Module 3 — AI Provider Gateway (M03)

Sole egress for external Vision + VLM APIs: clients, fallbacks, cost tracking, additive VLM evidence.

## Status

**PDR complete** (docs only). Rust `talos-ai-gateway` not started.

## Contents

| Path | Purpose |
|------|---------|
| [docs/pdr/M03-ai-provider-gateway.md](docs/pdr/M03-ai-provider-gateway.md) | Normative PDR |
| [docs/adr/0006-vlm-evidence-additive-only.md](docs/adr/0006-vlm-evidence-additive-only.md) | VLM evidence rules |
| [docs/adr/0007-provider-error-mapping.md](docs/adr/0007-provider-error-mapping.md) | HTTP → TalosError |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Module overview |
| [configs/ai_providers.toml](configs/ai_providers.toml) | Defaults (no secrets) |
| [wiki/](wiki/) | In-module wiki |

## Depends on

[Module 1 — Foundation & Core](../Module%201/)

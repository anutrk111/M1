# Getting Started (M03)

## Status

**PDR complete** (docs only). Rust `talos-ai-gateway` is not implemented yet.

## Read first

1. [PDR M03 — AI Provider Gateway](../docs/pdr/M03-ai-provider-gateway.md)
2. [ADR 0006 — VLM additive](../docs/adr/0006-vlm-evidence-additive-only.md)
3. [ADR 0007 — Error mapping](../docs/adr/0007-provider-error-mapping.md)
4. Config template: [`configs/ai_providers.toml`](../configs/ai_providers.toml)

## Secrets (when building later)

```bash
export TALOS__AI__OPENAI__API_KEY=...
# never commit keys
```

## Related

- Module 1 (types/errors): `Prime/Module 1/`
- Module 2 (intake): `Prime/Module 2/`

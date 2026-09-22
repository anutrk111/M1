# Getting Started (M12)

## Status

**PDR complete** (docs only). No IdP/OIDC or admin UI yet. `admin-gateway` is health-only until M12 build.

## Read

1. [PDR M12](../docs/pdr/M12-auth-admin-config.md)
2. [ADR 0034](../docs/adr/0034-rbac-roles-normative.md) — roles
3. [ADR 0035](../docs/adr/0035-secrets-env-only.md) — secrets
4. [ADR 0036](../docs/adr/0036-mfa-and-session-policy.md) — MFA/session
5. [ADR 0039](../docs/adr/0039-permission-based-authorization.md) — permissions
6. [ADR 0040](../docs/adr/0040-transactional-config-revision.md) — ConfigRevision
7. [`configs/auth.toml`](../configs/auth.toml) · [`configs/admin.toml`](../configs/admin.toml)

## Build later

`talos-auth` + `admin-gateway` auth/admin routes; persist audit via M10.

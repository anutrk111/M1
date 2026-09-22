# Module 12 — Auth, Admin & Config (M12)

Users, sessions, RBAC via permissions, MFA, admin settings, transactional config revisions. Decides who may act; owning modules decide what actions mean; M10 stores durable records.

## Status

**PDR complete** (docs only). IdP/OIDC, admin UI, and Rust auth not started. `admin-gateway` remains M01 health spine until M12 build.

## Contents

| Path | Purpose |
|------|---------|
| [docs/pdr/M12-auth-admin-config.md](docs/pdr/M12-auth-admin-config.md) | Normative PDR |
| [docs/adr/0034-rbac-roles-normative.md](docs/adr/0034-rbac-roles-normative.md) | Starter roles |
| [docs/adr/0035-secrets-env-only.md](docs/adr/0035-secrets-env-only.md) | Secret refs; redact |
| [docs/adr/0036-mfa-and-session-policy.md](docs/adr/0036-mfa-and-session-policy.md) | MFA, revoke, recovery |
| [docs/adr/0039-permission-based-authorization.md](docs/adr/0039-permission-based-authorization.md) | Permissions ≠ role-name checks |
| [docs/adr/0040-transactional-config-revision.md](docs/adr/0040-transactional-config-revision.md) | Atomic ConfigRevision |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Overview |
| [configs/auth.toml](configs/auth.toml) | Roles→permissions, MFA, session |
| [configs/admin.toml](configs/admin.toml) | Reloadable knob flags |
| [wiki/](wiki/) | In-module wiki |

## Depends on

[Module 1](../Module%201/) · [Module 10](../Module%2010/) (audit persistence). Authorizes [Module 9](../Module%209/) · [Module 11](../Module%2011/).

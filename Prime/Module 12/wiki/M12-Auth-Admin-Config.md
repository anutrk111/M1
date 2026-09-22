# M12 Auth, Admin & Config

Full text: [PDR](../docs/pdr/M12-auth-admin-config.md).

## Owns

AuthN/AuthZ, sessions, MFA policy, admin settings, ConfigRevision.

## Locked rules

- Authorize by permissions, not role-name checks
- Starter roles map to permissions in `auth.toml`
- `system_admin` ≠ automatic review authority
- Transactional ConfigRevision; invalid keeps previous
- Secrets: refs in TOML, values in env, redacted in logs/UI
- Hot reload only for explicit operational knobs
- Session revoke + privileged MFA recovery

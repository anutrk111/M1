# ADR 0036 — MFA and Session Policy (Expiry + Server-Side Revocation)

## Status

Accepted

## Context

Session expiry alone is insufficient after password reset, role removal, or account disable. MFA recovery endpoints can become the weakest path if treated like ordinary verify.

## Decision

1. Sessions have TTL and **server-side revocation**.
2. Revoke active sessions on: password reset, role removal/change, account disable, security-sensitive admin actions.
3. Default MFA required for `system_admin` and `review_lead` (config).
4. MFA **recovery/reset** is a higher-privilege, audited operation (e.g. requires `user.manage`), not ordinary MFA verification.
5. AuthN/Z events are append-only audit records (via M10 / AuthAuditStore).

## Consequences

- Compromised or stale sessions can be killed centrally.
- Recovery cannot silently bypass MFA without audit.

# ADR 0034 — Normative Starter Roles

## Status

Accepted; default role set refined by [ADR-0043](../../../Module%201/docs/adr/0043-operational-roles.md) (`super_admin`, `clerk`, `reviewing_officer`). Items 2–3 still apply.

## Context

M09 and M11 need a shared vocabulary of roles for documentation and default grants. Roles are convenience bundles; permissions authorize.

## Decision

1. Starter roles: `review_officer`, `review_lead`, `export_viewer`, `export_admin`, `system_admin`.
2. Default permission maps live in `configs/auth.toml` (see ADR-0039).
3. New roles may be added by composing permissions without changing M09/M11 authorization check sites.

## Consequences

- Docs and ops share one role catalog.
- APIs must not hard-code `if role == "review_lead"`.

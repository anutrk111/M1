# ADR 0035 — Secrets: References in TOML; Values in Env / Secret Manager

## Status

Accepted

## Context

Committed TOML with live API keys or passwords leaks into git history and mirrors.

## Decision

1. Committed TOML may hold **secret references/names only**.
2. Actual secrets come from environment or a secret manager.
3. Logs, API responses, and admin UI **redact** secret values.
4. Auth signing keys, provider API keys, and DB credentials are **not** hot-reloaded via generic config reload (restart required).

## Consequences

- Aligns with M01/M03 secret policy.
- Reduces accidental exposure in admin screens and traces.

# ADR 0039 — Permission-Based Authorization; system_admin ≠ Review Authority

## Status

Accepted

## Context

Hard-coding `if user.role == "review_lead"` in M09/M11 forces code changes for every new role. Granting `system_admin` automatic review power conflates infrastructure control with evidentiary authority.

## Decision

1. Authorization checks **permissions**, not role-name string equality.
2. Normative permissions include at least:
   - `review.read`, `review.act`, `review.override`
   - `export.read`, `export.audit`
   - `user.manage`, `config.read`, `config.write`
3. Roles map to permission sets (ADR-0034 / `auth.toml`).
4. **`system_admin` does not automatically receive review authority** (`review.act` / `review.override`). Dual power only via explicit assignment.

## Consequences

- Owning modules (M09/M11) ask “has permission X?”.
- Separation of duties is the default; deployments may opt into combined grants explicitly.

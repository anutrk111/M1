# ADR 0043 — Three Operational Roles

## Status

Accepted. Refines [ADR-0034](../../../Module%2012/docs/adr/0034-rbac-roles-normative.md). [ADR-0039](../../../Module%2012/docs/adr/0039-permission-based-authorization.md) is unchanged: authorization checks permissions, never role names.

## Context

The five starter roles in ADR-0034 are more granular than the streamlined operating model: one administrator, clerks who run intake and routine review, and officers who handle escalated cases only.

## Decision

1. The operational role set is `talos_types::Role { SUPER_ADMIN, CLERK, REVIEWING_OFFICER }`.
2. The default permission grants are:

   | Role | Default permissions |
   |------|---------------------|
   | `SUPER_ADMIN` | `user.manage`, `config.read`, `config.write`, `export.read`, `export.audit` |
   | `CLERK` | `intake.submit`, `review.read`, `review.act`, `export.read` |
   | `REVIEWING_OFFICER` | `review.read`, `review.act`, `review.override`, `export.read` |

3. `SUPER_ADMIN` has **no automatic review authority** (keeps the ADR-0034/M12 rule).
4. Only machine `REVIEW_REQUIRED` / ambiguous cases, or clerk escalations, reach a Reviewing Officer (M09 routing).
5. `intake.submit` is added to the normative permission set.
6. Deployments may still override grants in `Module 12/configs/auth.toml`.

## Consequences

- The ADR-0034 starter roles are retired as defaults. Deployments that need finer splits compose permissions without changing check sites.

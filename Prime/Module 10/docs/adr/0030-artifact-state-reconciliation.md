# ADR 0030 — ArtifactState Reconciliation; No False Persistence Success

## Status

Accepted

## Context

Partial writes create silent data loss:

- Postgres row written, then MinIO upload fails
- Object uploaded, then Postgres transaction fails

Reporting `Succeeded` / `AVAILABLE` in either case is unacceptable.

## Decision

1. Every blob+metadata write tracks **ArtifactState**:

```text
PENDING → OBJECT_WRITTEN → METADATA_COMMITTED → AVAILABLE
failure → RECONCILIATION_REQUIRED
```

2. Incomplete writes must **never** be reported as successful persistence / terminal success for that artifact.
3. Reconciliation jobs (later) repair or quarantine `RECONCILIATION_REQUIRED` rows.
4. Exact two-phase commit technology may vary; the **no false success** invariant is locked now.

## Consequences

- Workers and APIs can distinguish durable vs incomplete artifacts.
- Audit and export only promote `AVAILABLE` (or documented equivalent) artifacts.

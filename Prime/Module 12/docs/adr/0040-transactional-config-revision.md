# ADR 0040 — Transactional ConfigRevision Activation

## Status

Accepted

## Context

Partial application of decision thresholds or review flags can produce inconsistent pipeline behavior mid-batch.

## Decision

1. Config change pipeline:

```text
New Config → Parse → Schema Validation → Cross-field Validation
  → Create ConfigRevision → Atomic Activation → Audit Event
```

2. Invalid config: **reject** new revision; **keep previous active**.
3. Only explicitly reloadable operational knobs hot-reload; secrets require restart (ADR-0035).
4. Prefer every M08 MachineDecision to retain applicable `config_revision_id` / configuration fingerprint (M08/M10 store; M12 defines revision identity).

## Consequences

- No half-applied thresholds.
- Historical decisions remain interpretable against the config that governed them.

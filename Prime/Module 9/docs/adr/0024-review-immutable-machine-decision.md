# ADR 0024 — Immutable MachineDecision; Append-Only ReviewEvent; Separate ReviewedResult

## Status

Accepted

## Context

If human review overwrites the machine decision row, auditors cannot reconstruct what the AI said versus what the officer changed.

## Decision

1. Three layers are normative:
   - **MachineDecision** — immutable M08 output
   - **ReviewEvent** — append-only record of officer action + provenance
   - **ReviewedResult** — effective human-reviewed state derived from events
2. Human review **never overwrites** MachineDecision.
3. UI/export may display ReviewedResult as the effective view; MachineDecision remains stored and queryable.
4. Persistence of all three goes through M10 ports.

```text
MachineDecision → ReviewEvent (append-only) → ReviewedResult → M10 → M11
```

## Consequences

- Full before/after audit trail.
- Re-export and disputes can cite original machine output.

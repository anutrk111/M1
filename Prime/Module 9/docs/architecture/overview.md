# M09 Architecture Overview

**M09 Review Management** owns the human review workflow. It never overwrites MachineDecision.

## Principle

> Append what the officer did; keep what the machine said; persist both via M10.

## Layers

```text
MachineDecision (immutable)
 → ReviewEvent (append-only)
 → ReviewedResult (effective)
 → M10 → M11
```

## Docs

- [PDR M09](../pdr/M09-review-management.md)
- [ADR 0023](../adr/0023-review-actions-m01-only.md)
- [ADR 0024](../adr/0024-review-immutable-machine-decision.md)
- [ADR 0025](../adr/0025-correct-requires-detection-id.md)
- [ADR 0029](../adr/0029-review-claim-lease.md)

## Related

[Module 1](../../../Module%201/) · [Module 8](../../../Module%208/) · [Module 10](../../../Module%2010/)

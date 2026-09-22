# M10 Architecture Overview

**M10 Storage & Data** owns durability, integrity, indexing, and retrieval behind ports.

## Principle

> Persist immutably; hash identifies content; never claim success on incomplete writes.

## Data plane

```text
Ports → Adapters → PostgreSQL | Object Store | Timescale | Fixture
```

## Docs

- [PDR M10](../pdr/M10-storage-and-data.md)
- [ADR 0026](../adr/0026-postgres-system-of-record.md)
- [ADR 0027](../adr/0027-immutable-originals-hash-identity.md)
- [ADR 0028](../adr/0028-storage-ports-not-drivers.md)
- [ADR 0030](../adr/0030-artifact-state-reconciliation.md)

## Related

[Module 1](../../../Module%201/) · [Module 2](../../../Module%202/) · [Module 4](../../../Module%204/) · [Module 8](../../../Module%208/) · [Module 9](../../../Module%209/)

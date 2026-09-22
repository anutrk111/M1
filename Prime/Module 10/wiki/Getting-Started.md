# Getting Started (M10)

## Status

**PDR complete** (docs only). No migrations, MinIO wiring, or Rust adapters yet.

## Read

1. [PDR M10](../docs/pdr/M10-storage-and-data.md)
2. [ADR 0026](../docs/adr/0026-postgres-system-of-record.md) — store roles
3. [ADR 0027](../docs/adr/0027-immutable-originals-hash-identity.md) — hash identity
4. [ADR 0028](../docs/adr/0028-storage-ports-not-drivers.md) — ports
5. [ADR 0030](../docs/adr/0030-artifact-state-reconciliation.md) — ArtifactState
6. [`configs/storage.toml`](../configs/storage.toml)

## Build later

`talos-storage` adapters + Postgres migrations + object-store client. Credentials via environment only.

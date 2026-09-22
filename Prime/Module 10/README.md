# Module 10 — Storage & Data (M10)

Persistence for batches, frames, artifacts, decisions, review events, and metrics. Hash is integrity identity; logical paths are for humans. No false persistence success.

## Status

**PDR complete** (docs only). Rust adapters / migrations / MinIO wiring not started.

## Contents

| Path | Purpose |
|------|---------|
| [docs/pdr/M10-storage-and-data.md](docs/pdr/M10-storage-and-data.md) | Normative PDR |
| [docs/adr/0026-postgres-system-of-record.md](docs/adr/0026-postgres-system-of-record.md) | Postgres / Timescale / object store roles |
| [docs/adr/0027-immutable-originals-hash-identity.md](docs/adr/0027-immutable-originals-hash-identity.md) | Write-once + hash identity |
| [docs/adr/0028-storage-ports-not-drivers.md](docs/adr/0028-storage-ports-not-drivers.md) | Ports only |
| [docs/adr/0030-artifact-state-reconciliation.md](docs/adr/0030-artifact-state-reconciliation.md) | ArtifactState; no false success |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Overview |
| [configs/storage.toml](configs/storage.toml) | Placeholders; retention locked off |
| [wiki/](wiki/) | In-module wiki |

## Depends on

[Module 1](../Module%201/) types/enums. Serves M02/M04/M08/M09 via ports.

# Module 2 — Batch & File Intake (M02)

Turns folders / ZIP archives (+ optional CSV/XLSX sidecars) into M01 `IntakeEnvelope` v2.0 records and a reconciliable `BatchManifest`.

## Status

| Layer | Status |
|-------|--------|
| PDR / ADRs / configs / wiki | Complete |
| Rust crate `talos-intake` | **Implemented** (Prime workspace member) |
| HTTP upload API | Deferred (same rules apply when wired) |
| Durable queue sink | `NotImplemented` until queue module |

## Crate

```text
Prime/Module 2/crates/talos-intake
```

Member of the **Prime Cargo workspace** (`Prime/Cargo.toml`). Depends on `talos-types`, `talos-core` only for shared contracts.

```bash
cd Prime
cargo test -p talos-intake
cargo test --workspace --all-features
```

## Public API (summary)

- `import_folder` / `import_zip` → `ImportResult { manifest, outcome }`
- `BatchOutcome` — `Complete` | `PartialFailure` | `Rejected` (empty prefer `Err`)
- `FrameSink` + `InMemoryFrameSink`
- `BatchManifest` with reconciled `BatchCounts` accounting
- `load_intake_config` ← `configs/intake.toml`

## Staging lifecycle

M02 owns staging under `staging_root/<batch_id>/` for the duration of intake (copies / ZIP extract). **Cleanup is deferred** to the orchestrator / M10 — intake does not delete staging after a successful batch.

## Contents

| Path | Purpose |
|------|---------|
| [docs/pdr/M02-batch-and-file-intake.md](docs/pdr/M02-batch-and-file-intake.md) | Normative PDR |
| [docs/adr/0004-intake-metadata-join.md](docs/adr/0004-intake-metadata-join.md) | Metadata join |
| [docs/adr/0005-intake-no-silent-drop.md](docs/adr/0005-intake-no-silent-drop.md) | No silent drop |
| [docs/M02_IMPLEMENTATION_REPORT.md](docs/M02_IMPLEMENTATION_REPORT.md) | Implementation evidence |
| [docs/M02_HARDENING_REPORT.md](docs/M02_HARDENING_REPORT.md) | Hardening + workspace migration |
| [configs/intake.toml](configs/intake.toml) | Defaults |
| [crates/talos-intake](crates/talos-intake/) | Rust implementation |

## Depends on

[Module 1](../Module%201/)

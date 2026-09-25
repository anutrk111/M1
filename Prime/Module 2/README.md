# Module 2 — Batch & File Intake (M02)

Turns folders / ZIP archives (+ optional CSV/XLSX sidecars) into M01 `IntakeEnvelope` v2.0 records and a reconciliable `BatchManifest`.

## Status

| Layer | Status |
|-------|--------|
| PDR / ADRs / configs / wiki | Complete |
| Rust crate `talos-intake` | **Implemented** (Prime workspace member) |
| Upload intake (library `import_upload`) | Implemented; HTTP route deferred to M12 |
| Durable sink boundary (`DurableSink`, in-memory) | Implemented; real queue backend deferred to M10 |

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

- `import_folder` / `import_zip` / `import_upload(Vec<UploadedFile>, …)` → `ImportResult { manifest, outcome }`
- `import_source(IntakeSource, …, Option<&RecoveryContext>)` — shared entry point; recovery re-run for an existing batch (`RecoveryContext::from_manifest`, keyed on `(relative_path, sha256)`)
- `BatchOutcome` — `Complete` | `PartialFailure` | `Rejected` (empty prefer `Err`)
- `FrameSink` + `InMemoryFrameSink`
- `DurableSink` (`offer(envelope, IdempotencyKey) -> Result<SinkAck, TalosError>`) + `InMemoryDurableSink`, `UnconfiguredDurableSink`, `DurableSinkAdapter<D>: FrameSink`
- `BatchManifest` with reconciled `BatchCounts` accounting (incl. `skipped_already_accepted`)
- `StagingPolicy` + pure `plan_cleanup(manifest, staging_root, policy) -> CleanupPlan`
- `load_intake_config` ← `configs/intake.toml`

## Staging lifecycle

M02 owns staging under `staging_root/<batch_id>/` for the duration of intake (copies / ZIP extract / uploads). **Cleanup is deferred** to the orchestrator / M10 — intake never deletes staging. `plan_cleanup` only reports what *would* be deleted under `staging_policy` (`retain` default | `plan_only` | `plan_batch_dir`).

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

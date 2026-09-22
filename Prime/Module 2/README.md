# Module 2 — Batch & File Intake (M02)

Turns folders / ZIP archives (+ optional CSV/XLSX sidecars) into M01 `IntakeEnvelope` v2.0 records and a reconciliable `BatchManifest`.

## Status

| Layer | Status |
|-------|--------|
| PDR / ADRs / configs / wiki | Complete |
| Rust crate `talos-intake` | **Implemented** (workspace member via Module 1) |
| HTTP upload API | Deferred (same rules apply when wired) |
| Durable queue sink | `NotImplemented` until queue module |

## Crate

```text
Prime/Module 2/crates/talos-intake
```

Integrated into the **Module 1 Cargo workspace** (path member / symlink). Depends on `talos-types`, `talos-core` only for shared contracts.

```bash
cd "Prime/Module 1"
cargo test -p talos-intake
```

## Public API (summary)

- `import_folder` / `import_zip`
- `FrameSink` + `InMemoryFrameSink`
- `BatchManifest` with `IntakeStatus` accounting
- `load_intake_config` ← `configs/intake.toml`

## Contents

| Path | Purpose |
|------|---------|
| [docs/pdr/M02-batch-and-file-intake.md](docs/pdr/M02-batch-and-file-intake.md) | Normative PDR |
| [docs/adr/0004-intake-metadata-join.md](docs/adr/0004-intake-metadata-join.md) | Metadata join |
| [docs/adr/0005-intake-no-silent-drop.md](docs/adr/0005-intake-no-silent-drop.md) | No silent drop |
| [docs/M02_IMPLEMENTATION_REPORT.md](docs/M02_IMPLEMENTATION_REPORT.md) | Implementation evidence |
| [configs/intake.toml](configs/intake.toml) | Defaults |
| [crates/talos-intake](crates/talos-intake/) | Rust implementation |

## Depends on

[Module 1](../Module%201/)

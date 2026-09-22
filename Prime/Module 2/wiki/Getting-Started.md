# Getting Started (M02)

M02 is **docs-only** in this step. There is no Cargo workspace or `talos-intake` crate to build yet.

## Where the normative docs live

| Document | Path |
|----------|------|
| PDR | [`docs/pdr/M02-batch-and-file-intake.md`](../docs/pdr/M02-batch-and-file-intake.md) |
| Architecture | [`docs/architecture/overview.md`](../docs/architecture/overview.md) |
| ADR 0004 — metadata join | [`docs/adr/0004-intake-metadata-join.md`](../docs/adr/0004-intake-metadata-join.md) |
| ADR 0005 — no silent drop | [`docs/adr/0005-intake-no-silent-drop.md`](../docs/adr/0005-intake-no-silent-drop.md) |
| Config defaults | [`configs/intake.toml`](../configs/intake.toml) |
| Wiki mirror of PDR | [[M02-Batch-and-File-Intake]] |

## Prerequisites (when implementation lands)

- Rust toolchain (aligned with Module 1)
- Module 1 crates available via path dependency (`talos-types`, `talos-core`, …)

## Read next

1. [[M02-Batch-and-File-Intake]] — intake modes, join rules, manifest, `FrameSink`
2. Module 1 Getting Started / architecture for envelope schema and pipeline contract

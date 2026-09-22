# M02 Architecture Overview — Batch & File Intake

Short overview for **Module 2**. Full hexagonal map and Stage 0–7 pipeline live in [Module 1 architecture](../../Module%201/docs/architecture/overview.md). Normative intake rules are in the [M02 PDR](../pdr/M02-batch-and-file-intake.md).

## Role

M02 turns raw batch inputs (image folders, ZIP archives, HTTP uploads, optional CSV/XLSX sidecars) into validated, content-addressed **`IntakeEnvelope`** records (JSON schema v2.0) ready for the M01 Stage 0–7 pipeline.

```text
Folder / ZIP / Upload ──► M02 Intake ──► IntakeEnvelope + BatchManifest
                              │
                              └── FrameSink ──► M01 pipeline (later)
```

M02 **produces** envelopes defined by M01. It does **not** redefine schemas, run inference, call AI providers, or own long-term storage.

## Depends on M01

| From M01 | Used by M02 |
|----------|-------------|
| `IntakeEnvelope`, `SourceKind`, IDs | Envelope emission |
| SHA-256 / ID helpers | Content addressing |
| `TalosError` taxonomy | Validation, permanent, transient, `NotImplemented` |
| Tracing / health conventions | `batch.run` spans, metrics naming |

See [M01 Foundation & Core PDR](../../Module%201/docs/pdr/M01-foundation-and-core.md).

## Locked decisions (ADRs)

| ADR | Decision |
|-----|----------|
| [0004 — metadata join](../adr/0004-intake-metadata-join.md) | Relative-path primary + basename fallback |
| [0005 — no silent drop](../adr/0005-intake-no-silent-drop.md) | Accepted ⇒ sink success or `FailedSink` on manifest |

## Config

Defaults: [`configs/intake.toml`](../../configs/intake.toml) (PDR §11). Override with `TALOS__INTAKE__*`.

## Implementation status

Docs and config defaults only. Future crate: `Prime/Module 2/crates/talos-intake` (depends on Module 1 path crates).

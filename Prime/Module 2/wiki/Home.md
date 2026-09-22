# Module 2 Wiki — Batch & File Intake

**Talos-RS** — Automated Vehicle Identification & HSRP Audit Engine (100% Rust / Tokio).

This wiki covers Module **M02 Batch & File Intake**. Foundation types and the Stage 0–7 pipeline live in Module 1.

## Quick links

| Page | Description |
|------|-------------|
| [[Getting-Started]] | Where docs live (docs-only; no cargo yet) |
| [[M02-Batch-and-File-Intake]] | Full M02 PDR (wiki mirror) |

## Product posture

- **API-first**, batch / file intake (folders, ZIP, CSV/XLSX)
- Produces M01 `IntakeEnvelope` (schema v2.0); does not run Stages 0–7
- **No silent drops** — accepted frames reach `FrameSink` or are recorded as `FailedSink`
- Metadata join: relative path primary, basename fallback

## Related

- Normative PDR (repo): [`docs/pdr/M02-batch-and-file-intake.md`](../docs/pdr/M02-batch-and-file-intake.md)
- Module 1: [`Prime/Module 1/`](../../Module%201/)

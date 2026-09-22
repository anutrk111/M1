# Module 2 — Batch & File Intake (M02)

Turns folders, ZIP archives, and HTTP uploads into M01 `IntakeEnvelope` records (JSON schema v2.0), with optional CSV/XLSX metadata join and a batch manifest.

## Status

**Docs complete** (PDR, ADRs, wiki, config defaults). Rust crates are **not** implemented yet.

## Folder layout

```text
Prime/Module 2/
  README.md
  docs/
    pdr/M02-batch-and-file-intake.md   # Normative PDR
    adr/0004-intake-metadata-join.md    # Relative-path join + basename fallback
    adr/0005-intake-no-silent-drop.md  # Accepted ⇒ sink or FailedSink
    architecture/overview.md           # M02-focused overview
  wiki/
    Home.md
    _Sidebar.md
    Getting-Started.md                 # Docs-only; no cargo yet
    M02-Batch-and-File-Intake.md       # Wiki mirror of the PDR
  configs/
    intake.toml                        # Defaults from PDR §11
  tests/
    fixtures/.gitkeep                  # Future intake fixtures
```

## Docs

| Document | Path |
|----------|------|
| **PDR (normative)** | [docs/pdr/M02-batch-and-file-intake.md](docs/pdr/M02-batch-and-file-intake.md) |
| Architecture overview | [docs/architecture/overview.md](docs/architecture/overview.md) |
| ADR 0004 — metadata join | [docs/adr/0004-intake-metadata-join.md](docs/adr/0004-intake-metadata-join.md) |
| ADR 0005 — no silent drop | [docs/adr/0005-intake-no-silent-drop.md](docs/adr/0005-intake-no-silent-drop.md) |
| Wiki (in-repo) | [wiki/](wiki/) |

## Depends on

[Module 1 — Foundation & Core](../Module%201/) (`talos-types`, `talos-core` utilities / errors, pipeline handoff).

## Later (not in this folder yet)

- `crates/talos-intake`
- HTTP / CLI intake wired to `FrameSink`
- Durable queue adapters (NATS / RabbitMQ)

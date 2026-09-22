# Module 11 — Export & Reporting (M11)

Presentation-layer exports and summaries. Not a decision engine. Reads via M10 ports; never mutates stored source values.

## Status

**PDR complete** (docs only). Rust export workers / spreadsheet libs not started.

## Contents

| Path | Purpose |
|------|---------|
| [docs/pdr/M11-export-and-reporting.md](docs/pdr/M11-export-and-reporting.md) | Normative PDR |
| [docs/adr/0031-export-machine-vs-effective.md](docs/adr/0031-export-machine-vs-effective.md) | machine / effective / both + effective_source |
| [docs/adr/0032-export-via-m10-ports.md](docs/adr/0032-export-via-m10-ports.md) | Read-only via M10 |
| [docs/adr/0033-export-formats-csv-xlsx.md](docs/adr/0033-export-formats-csv-xlsx.md) | CSV / XLSX / JSONL |
| [docs/adr/0037-export-manifest-reproducibility.md](docs/adr/0037-export-manifest-reproducibility.md) | ExportManifest |
| [docs/adr/0038-spreadsheet-safe-serialization.md](docs/adr/0038-spreadsheet-safe-serialization.md) | Formula-injection-safe serialize |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Overview |
| [configs/export.toml](configs/export.toml) | Defaults |
| [wiki/](wiki/) | In-module wiki |

## Depends on

[Module 1](../Module%201/) · [Module 8](../Module%208/) · [Module 9](../Module%209/) · [Module 10](../Module%2010/) · [Module 12](../Module%2012/) (permissions)

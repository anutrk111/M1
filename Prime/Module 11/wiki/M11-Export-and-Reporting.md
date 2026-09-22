# M11 Export & Reporting

Full text: [PDR](../docs/pdr/M11-export-and-reporting.md).

## Owns

CSV / XLSX / JSONL exports, summaries, ExportManifest.

## Locked rules

- Presentation only — not a decision engine
- Views: machine / effective / both; `effective_source` on effective rows
- Read via M10 ports; never mutate storage
- Spreadsheet-safe serialize without mutating source
- ExportManifest with SHA-256 + stable ordering
- `on_reconciliation_required = flag` by default

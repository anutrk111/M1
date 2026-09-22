# M11 Architecture Overview

**M11 Export & Reporting** is presentation-only over M10 data.

## Principle

> Show what was stored — never decide or silently rewrite it.

## Flow

```text
M12 AuthZ → M11 Query → M10 Ports → CSV/XLSX/JSONL + ExportManifest
```

## Docs

- [PDR M11](../pdr/M11-export-and-reporting.md)
- [ADR 0031](../adr/0031-export-machine-vs-effective.md)
- [ADR 0032](../adr/0032-export-via-m10-ports.md)
- [ADR 0033](../adr/0033-export-formats-csv-xlsx.md)
- [ADR 0037](../adr/0037-export-manifest-reproducibility.md)
- [ADR 0038](../adr/0038-spreadsheet-safe-serialization.md)

## Related

[Module 9](../../../Module%209/) · [Module 10](../../../Module%2010/) · [Module 12](../../../Module%2012/)

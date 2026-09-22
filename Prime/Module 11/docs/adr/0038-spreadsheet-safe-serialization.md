# ADR 0038 — Spreadsheet-Safe Serialization Without Mutating Storage

## Status

Accepted

## Context

User-controlled plate text and review notes may start with `=`, `+`, `-`, or `@`, which spreadsheet apps treat as formulas (injection risk). Mutating stored values to “fix” them would corrupt audit lineage.

## Decision

1. CSV/XLSX **serialization** applies spreadsheet-safe escaping/prefixing for consumer files.
2. M11 **does not mutate** stored source values in M10.
3. JSONL may retain raw strings; document consumer responsibility if opened in spreadsheets.
4. Safety is a presentation concern, not a decision/normalization engine.

## Consequences

- Safe downloads without rewriting MachineDecision / ReviewEvent content.
- Stored truth remains authoritative for re-export and review UI.

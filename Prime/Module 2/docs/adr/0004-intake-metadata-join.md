# ADR 0004 — Intake Metadata Join

## Status

Accepted

## Context

M02 optionally joins CSV/XLSX sidecar rows to discovered images so `FrameMetadata` can be populated on each `IntakeEnvelope`. Join must be deterministic across folder, ZIP, and upload sources without aborting an entire batch for a single bad row.

## Decision

1. **Primary key:** relative path within the batch root, using `/` separators, matched case-sensitively to the sidecar `path` / `file` / `relative_path` value after normalization (strip leading `./`).
2. **Fallback:** if no relative-path match, join on **basename** (file name only). If multiple images share a basename, basename join applies only when exactly one candidate remains; otherwise leave metadata null and record a warning.
3. **Unmatched images:** allowed — envelope emits with null / empty metadata fields.
4. **Malformed rows:** fail that row (manifest warning/error); continue the batch unless `fail_batch_on_metadata_errors = true` (default **false**).

## Consequences

- Sidecar absence is valid; no metadata is required for acceptance.
- Ambiguous basename collisions never invent a join; they warn and leave metadata null.
- Implementers must normalize join keys the same way for folder and ZIP-extracted trees.

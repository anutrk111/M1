# ADR 0012 — OCR Via M03; Grammar Local

## Status

Accepted

## Context

Stage 3 needs remote OCR models. Stage 4 Indian registration grammar must be deterministic, auditable, and independent of provider quirks.

## Decision

1. M06 Stage 3 remote OCR goes **only** through M03 `run_ocr`.
2. Backends: `fixture` (CI) or `ai_api` (M03). No ad-hoc HTTP in M06.
3. Stage 4 grammar runs **in-process** in M06 (deterministic rules). It is not an M03 call.
4. Missing credentials / unimplemented OCR → `NotImplemented` or `Config`; never invent plate text.
5. Prefer M04 `api_prepared` (then enhanced, then original crop) as the OCR input artifact.

## Consequences

- Cost/retries stay in M03 for OCR.
- Grammar behavior is versionable and testable without network.
- High OCR confidence never justifies skipping grammar (see ADR-0019).

# ADR 0019 — Evidence-Supported Grammar (No Plate Invention)

## Status

Accepted

## Context

Grammar must normalize and validate registration candidates without inventing a new plate identity. Blind `O→0` style rewrites without OCR support create false confidence.

## Decision

1. Stage 4 grammar is for **evidence-supported normalization and validation**, not for creating a new plate.
2. Corrections such as `O↔0` apply **only** when OCR alternatives and/or character-level evidence support them.
3. High OCR confidence **never** bypasses grammar.
4. GrammarResult statuses (normative):
   - `VALID`
   - `KNOWN_INVALID`
   - `UNKNOWN_TO_REGISTRY`
5. `GrammarResult` carries the **same** `DetectionId` as its OCR input.
6. `NoPlateDetected` paths skip OCR/grammar; do not invent text.

## Consequences

- Invalid high-confidence OCR stays visible to M08 as `KNOWN_INVALID` (or similar).
- M08 eligibility can hard-constrain on grammar status before ranking.

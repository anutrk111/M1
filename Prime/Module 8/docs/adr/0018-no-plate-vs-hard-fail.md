# ADR 0018 — NoPlateDetected vs Hard-Fail

## Status

Accepted

## Context

M04 may emit typed `NoPlateDetected`. That is not an M06/M07 crash; M08 must map it via policy.

## Decision

1. `NoPlateDetected` is a typed upstream signal, not an OCR/HSRP exception.
2. When `hard_fail_on_no_plate = true`, force a non-AutoApproved terminal path (typically `ReviewRequired` or equivalent policy outcome).
3. Never invent plate text or DetectionIds to “fill” the gap.
4. Document outcome explicitly in decision provenance.

## Consequences

- Empty-plate frames remain auditable.
- M06/M07 stay free of decision authority.

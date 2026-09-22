# ADR 0016 — Config-Driven Decision Thresholds

## Status

Accepted

## Context

Hardcoding approve/secondary cutoffs in code makes field tuning and audit difficult. Early numeric defaults must not be mistaken for validated production accuracy.

## Decision

1. `auto_approve_min` and `secondary_min` come from config (`decision.toml`), with M01-compatible keys.
2. Defaults `0.90` / `0.80` are **starting configuration defaults only**, not certified accuracy thresholds.
3. Code may mirror defaults only as config fallbacks when files omit keys; runtime must still load/validate config.
4. Changing thresholds requires config change + audit trail, not a silent code constant.

## Consequences

- Ops can tune without recompiling.
- Docs and dashboards must label defaults as starting points.

# ADR 0022 — Observation Cluster ≠ Same Vehicle

## Status

Accepted

## Context

M05 suppresses overlapping boxes **within one frame**. M08 Stage 6 operates across imported frames/files. Claiming “same vehicle” from weak signals is an unsupported semantic overreach in early versions.

## Decision

1. Stage 6 produces **duplicate observation relationships / clusters**, not vehicle identity.
2. Signals may include image hash, crop similarity, plate hypothesis, and available metadata.
3. Config `claim_same_vehicle = false` for early versions; docs must not assert same-vehicle identity.
4. Stage 6 ≠ M05 in-frame IoU suppression.

```text
M05: same frame + overlapping boxes → Detection Suppression
M08 Stage 6: different imported frames/files → Observation Deduplication → cluster
```

## Consequences

- Safer language for audits and future tracking modules.
- Clusters remain useful for reducing duplicate AutoApprove noise without overclaiming.

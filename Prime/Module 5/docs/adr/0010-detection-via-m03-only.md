# ADR 0010 — Detection Via M03 Only

## Status

Accepted

## Context

Stage 1 needs remote vision models. Scattering HTTP clients across modules breaks retries, cost tracking, and error mapping.

## Decision

1. M05 Stage 1 remote inference goes **only** through M03 `detect_vehicles_plates`.
2. Backends: `fixture` (CI) or `ai_api` (M03). No ad-hoc `reqwest` in M05.
3. Provider transport errors follow ADR-0007 (`Transient` / `Validation` / etc.).
4. Missing credentials / unimplemented provider → `NotImplemented` or `Config`; never invent bounding boxes.
5. Detection uses the **original frame**, not plate crops.

## Consequences

- Cost and failover stay centralized in M03.
- Fixture path proves Stage 1 without network.

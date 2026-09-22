# ADR 0014 — HSRP Via M03; Evidence Only

## Status

Accepted

## Context

Stage 5 needs remote vision cues for HSRP-related marks. Declaring legal compliance inside M07 would conflate observation with adjudication and duplicate M08 authority.

## Decision

1. M07 Stage 5 remote analysis goes **only** through M03 `analyze_hsrp`.
2. Backends: `fixture` (CI) or `ai_api` (M03). No ad-hoc HTTP in M07.
3. M07 emits **observations + confidence**, not “legal HSRP” / “illegal plate” verdicts.
4. Missing credentials / unimplemented provider → `NotImplemented` or `Config`; never invent registration text or fake cues.
5. Prefer M04 `api_prepared` crop as the analysis artifact.

## Consequences

- Legal/compliance fusion stays in M08 (and human review).
- Cost and failover stay in M03.

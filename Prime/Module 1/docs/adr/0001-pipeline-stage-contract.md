# ADR 0001 — Pipeline Stage Contract

## Status

Accepted

## Context

Talos-RS processes each frame through an ordered Stages 0–7 chain. Modules M04–M08 implement stage logic; M01 owns orchestration.

## Decision

1. Every stage implements `Stage` with `id`, `name`, and `async process`.
2. `process` returns `StageStatus::{Continue, SkipRemaining, Halt}` or `TalosError`.
3. `Pipeline::run` executes stages in strictly increasing `StageId` order and records `timings_ms`.
4. **Continue** — proceed to the next stage.
5. **SkipRemaining** — stop further stages; caller finalizes terminal status/decision as needed. No invented OCR/detections.
6. **Halt** — set `FrameTerminalStatus::Halted` and return `Ok(())` without inventing outputs.
7. **NotImplemented** — set `FailedNotImplemented` and return the error; never fabricate plate text.

## Consequences

- Stage order cannot be changed by feature modules.
- Workers own retry loops for `Transient` errors.
- Fixture stages may implement the same contract for offline CI.

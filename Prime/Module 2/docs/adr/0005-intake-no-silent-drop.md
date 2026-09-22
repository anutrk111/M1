# ADR 0005 — Intake No Silent Drop

## Status

Accepted

## Context

M02 produces validated `IntakeEnvelope` records and hands them to a `FrameSink`. Callers (HTTP, CLI, workers) must not believe a batch succeeded while accepted frames disappeared without a terminal record.

## Decision

1. Once a frame is classified **`Accepted`** (envelope built), it must either:
   - Successfully reach `FrameSink::submit`, or
   - Be recorded on the `BatchManifest` as **`FailedSink`** with error detail.
2. Never acknowledge HTTP/CLI success for a batch while accepted frames are missing from both the sink and the manifest.
3. Rejected / skipped frames (`RejectedValidation`, `RejectedPermanent`, `SkippedDuplicate`) must also appear on the manifest with a terminal intake status — no silent omission of discovered candidates that were evaluated.

## Consequences

- Manifest completeness is part of the intake contract, not an optional diagnostic.
- Sink failures are visible and countable (`talos_intake_frames_total{status=…}`).
- Durable-queue adapters that are not wired return `NotImplemented` and surface as `FailedSink` / batch error rather than pretending success.

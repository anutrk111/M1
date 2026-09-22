# ADR 0003 — No Local GPU by Default

## Status

Accepted

## Context

Current Talos-RS generation is API-first batch processing with external Vision/VLM providers (M03). On-box ONNX/TensorRT is a future extension, not the default.

## Decision

1. Production stage backend default is `ai_api` (via M03) once providers are wired; CI/local spine uses `fixture`.
2. `local_gpu_ext` remains an explicit backend selector that must return `TalosError::NotImplemented` until models and runtime are enabled.
3. Live camera ingest and VAHAN are likewise extension points (see product posture in architecture overview).

## Consequences

- No pretend GPU inference in the repository.
- `inference-runtime` (when scaffolded) is fail-loud until real artifacts exist.
- Fixture pipeline proves M01 contracts without network or GPU.

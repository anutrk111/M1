# PDR M01 — Foundation & Core

| Field | Value |
|-------|-------|
| **Module ID** | M01 |
| **Module name** | Foundation & Core |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) |
| **Architecture** | Hexagonal modular monorepo; M01 is the central dependency hub |
| **Runtime** | Rust 2021, Tokio (async) |
| **Related docs** | [Architecture overview](../architecture/overview.md) |

---

## 1. Purpose

M01 provides the shared runtime spine for Talos-RS so every other module (M02–M12):

1. Speaks the same **types and JSON schemas**
2. Fails the same way via a shared **error taxonomy**
3. Is configured the same way (**TOML + ENV**, validated at boot)
4. Executes the same **Stage 0–7 pipeline contract**
5. Emits the same **tracing, health, and lifecycle** semantics

M01 owns cross-cutting contracts and orchestration framework code. It does **not** own business adapters (intake, AI providers, storage drivers, auth providers, or export formats).

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **Primary mode** | API-first, **batch / file intake** (folders, ZIP uploads, CSV/XLSX metadata) |
| **Not live streams** | No live camera access in the current generation |
| **No VAHAN (current)** | External transport-registry verification is out of scope; types reserved only |
| **No local GPU (current)** | Vision / OCR / HSRP run via **external AI Provider Gateway (M03)** |
| **Local GPU** | Extension hook only (`local_gpu_ext`); must return `NotImplemented` until explicitly enabled |
| **Evidence model** | Decisions are **evidence-driven**, not OCR-alone |

M01 must remain valid if live cameras, VAHAN, or local GPU are added later. Those capabilities are extension points, not defaults.

---

## 3. Scope

### 3.1 In scope

| Capability | Description |
|------------|-------------|
| Common types & schemas | Batch/job/frame IDs, plate candidates, evidence, decisions, JSON schema v2.0 envelopes |
| Configuration | TOML files + environment overlays; fail-closed validation at boot |
| Error handling | Typed `TalosError` taxonomy; retryability class; no silent frame drops |
| Pipeline framework | `Stage` trait, `FrameContext`, ordered Stages 0–7 orchestration |
| Tracing & logging | Structured `tracing`; correlation across batch → frame → stage |
| Health & readiness | `/healthz` and `/readyz` semantics for all binaries |
| Runtime lifecycle | Bootstrap, graceful shutdown, signal handling |
| Utilities | SHA-256 / BLAKE3 helpers, time, ID generation |

### 3.2 Out of scope (owned elsewhere)

| Module | Owns |
|--------|------|
| M02 | ZIP / folder / upload intake adapters |
| M03 | Vision / VLM HTTP clients, fallbacks, cost tracking |
| M04–M07 | Concrete image, vision, OCR, HSRP algorithms (they *implement* M01 stage traits) |
| M08 | Fusion policy details beyond shared outcome enums and threshold config keys |
| M09 | Review officer workflow UI / APIs |
| M10 | PostgreSQL / TimescaleDB / MinIO persistence |
| M11 | CSV / XLSX export and reporting |
| M12 | Users, RBAC, MFA, admin settings |

### 3.3 Non-goals

- Calling external AI APIs directly from M01
- Persisting to databases from M01
- Fake “success” stubs that invent plate text or detections
- Hardcoding decision thresholds in Rust (defaults live in config; code may mirror defaults only as config fallbacks)
- Silently discarding accepted batch frames under back-pressure

---

## 4. Goals and success criteria

### 4.1 Goals

1. One `FrameContext` flows Stages 0→7 without module-specific glue types.
2. Any stage may return `NotImplemented` loudly when a backend is missing.
3. Evidence is typed: `VisualObservation` vs `ExternalDatabaseVerification` (latter unused until authorized adapters exist; still defined now).
4. All binaries share identical health / lifecycle / tracing bootstrap.

### 4.2 Acceptance criteria

- [ ] Crates `talos-types`, `talos-config`, and `talos-core` compile and are depended on by every app binary.
- [ ] `Pipeline::run` executes Stages 0–7 in order; abort / skip policy is documented and tested.
- [ ] Config load fails closed on invalid TOML or env.
- [ ] Fixture pipeline test produces schema-valid decision JSON without GPU or network.
- [ ] ADRs record that local GPU, live camera, and VAHAN are extension points only.

---

## 5. Crate mapping (implementation binding)

| M01 concern | Crate | Rules |
|-------------|-------|-------|
| Types & schemas | `crates/talos-types` | No Tokio; `serde` only; no I/O |
| Configuration | `crates/talos-config` | Layered load (defaults → TOML → `TALOS__*` env); validated structs |
| Pipeline, errors, lifecycle | `crates/talos-core` | Depends on `talos-types` + `talos-config` |
| Tracing init / health helpers | `crates/talos-core` (`runtime` module) | Apps call `talos_core::runtime::init` / `bootstrap` |
| Metrics name conventions | `crates/observability` | M01 defines naming conventions; other modules register series |

**Application binaries** (`talos-api`, `talos-worker`, `review-api`, `admin-gateway`) depend on M01 crates. They must not redefine schemas, decision enums, or error types.

```text
apps/* ──depends──► talos-core ──depends──► talos-config
                         │                      │
                         └──────depends─────────┴──► talos-types
```

---

## 6. Common types and schemas (normative)

### 6.1 Identity

| Type | Meaning | Serialization |
|------|---------|---------------|
| `BatchId` | One intake unit (folder, ZIP, or upload set) | String (ULID or UUID v7) |
| `JobId` | Optional sub-work unit inside a batch | String |
| `FrameId` | One image / frame under processing | String |
| `CameraId` | Optional metadata from CSV/XLSX — **not** a live feed handle | String or null |
| `TraceId` | Correlation ID for logs and spans | String |

IDs are opaque strings in JSON. Generators live in M01 utilities.

### 6.2 Intake envelope (JSON schema v2.0)

Normative conceptual shape (M02 produces; M01 defines):

```json
{
  "schema_version": "2.0",
  "batch_id": "01J…",
  "frame_id": "01J…",
  "source": {
    "kind": "folder",
    "path_or_key": "/data/batch-001/img_00042.jpg"
  },
  "image": {
    "content_type": "image/jpeg",
    "sha256": "…",
    "bytes_ref": "object://… or file://…"
  },
  "metadata": {
    "camera_id": null,
    "captured_at": null,
    "location": null
  }
}
```

`source.kind` ∈ { `folder`, `zip`, `upload` }.

### 6.3 Pipeline artifacts on `FrameContext`

| Artifact | Stage | Producing module |
|----------|-------|------------------|
| `ImageQualityReport` | 0 | M04 |
| `Detections` (vehicles / plates, bboxes, scores) | 1 | M05 via M03 |
| `RectifiedPlate` (crop refs / geometry) | 2 | M04 |
| `OcrHypothesis[]` (text, per-char confidence) | 3 | M06 via M03 |
| `GrammarResult` (Indian registration validation) | 4 | M06 (deterministic preferred) |
| `HsrpEvidence` (IND mark, hologram cues, geometry/color flags) | 5 | M07 via M03 |
| `DedupResult` (batch-scoped hash / track hits) | 6 | M08 helpers / M10 |
| `FusedDecision` (outcome, scores, evidence list) | 7 | M08 |

`FrameContext` also carries: intake envelope fields, `timings_ms` per stage, accumulated `Evidence[]`, and terminal status once set.

### 6.4 Evidence (normative)

```rust
pub enum EvidenceKind {
    VisualObservation,
    ExternalDatabaseVerification,
}

pub struct Evidence {
    pub kind: EvidenceKind,
    pub source: String,
    pub payload: serde_json::Value,
    pub confidence: f32, // 0.0..=1.0
}
```

**Separation rule (mandatory):**

- The visual pipeline (Stages 0–7 default path) may only append `EvidenceKind::VisualObservation`.
- `ExternalDatabaseVerification` may only be written by authorized integration adapters (future; e.g. VAHAN-class). M01 forbids visual stages from emitting this kind.
- VLM / secondary AI (M03) contributes **additional visual evidence**; it must not silently override deterministic grammar or hard-fail flags.

### 6.5 Decision outcomes (shared with M08)

| Outcome | Default rule (config-driven) |
|---------|------------------------------|
| `AutoApproved` | `fused_confidence >= auto_approve_min` (default **0.90**) |
| `SecondaryVerification` | `secondary_min <= fused_confidence < auto_approve_min` (default **0.80–0.90**) |
| `ReviewRequired` | below `secondary_min`, or any configured hard-fail flag |

**Review actions** (typed in M01; consumed by M09):

`Verify` | `Correct` | `Reject` | `Unreadable`

### 6.6 Decision JSON (minimum export / API shape)

```json
{
  "schema_version": "2.0",
  "batch_id": "…",
  "frame_id": "…",
  "plate_text": "…",
  "grammar_ok": true,
  "hsrp_score": 0.0,
  "fused_confidence": 0.0,
  "outcome": "AutoApproved",
  "evidence": [],
  "stages_ms": {
    "0": 0,
    "1": 0,
    "2": 0,
    "3": 0,
    "4": 0,
    "5": 0,
    "6": 0,
    "7": 0
  }
}
```

`outcome` ∈ { `AutoApproved`, `SecondaryVerification`, `ReviewRequired` }.

### 6.7 Frame terminal status

M01 defines `FrameTerminalStatus` so accepted work cannot vanish:

| Status | Meaning |
|--------|---------|
| `Succeeded` | Pipeline completed; decision emitted |
| `FailedValidation` | Bad input / schema |
| `FailedPermanent` | Corrupt image, unsupported codec, etc. |
| `FailedTransientExhausted` | Retries exhausted |
| `FailedNotImplemented` | Required backend missing |
| `Halted` | Stage returned `Halt` |

Persistence of terminal status is M10’s responsibility; M01 defines the enum and requires workers to set it before acknowledging completion.

---

## 7. Pipeline framework (Stages 0–7)

### 7.1 Contracts

```rust
#[async_trait]
pub trait Stage: Send + Sync {
    fn id(&self) -> StageId; // S0 .. S7
    fn name(&self) -> &'static str;
    async fn process(&self, ctx: &mut FrameContext) -> Result<StageStatus, TalosError>;
}

pub enum StageStatus {
    Continue,
    SkipRemaining,
    Halt,
}

pub struct Pipeline { /* ordered Vec<Arc<dyn Stage>> */ }

impl Pipeline {
    pub async fn run(&self, ctx: &mut FrameContext) -> Result<(), TalosError>;
}
```

### 7.2 Stage registry

| Stage | Name | Primary module | M01 duty |
|-------|------|----------------|----------|
| 0 | Image Quality Assessment | M04 | Invoke; record latency |
| 1 | Vehicle + Plate Detection | M05 via M03 | Invoke; record latency |
| 2 | Plate Rectification & Enhancement | M04 | Invoke; record latency |
| 3 | OCR | M06 via M03 | Invoke; record latency |
| 4 | Indian Registration Grammar | M06 | Prefer deterministic in-process |
| 5 | HSRP Evidence Engine | M07 via M03 | Invoke; record latency |
| 6 | Observation Deduplication / Clustering | M08 / M10 helpers | Batch-scoped store injection |
| 7 | Confidence Fusion & Decision | M08 | Apply **config** thresholds |

M01 orchestrates only. Algorithmic logic lives in M04–M08 (and M03 for remote vision).

### 7.3 Execution policy

1. Stages run **strictly in order 0→7** unless a stage returns `SkipRemaining` or `Halt`.
2. `Continue` — proceed to the next stage.
3. `SkipRemaining` — stop further stages; caller must still finalize terminal status / decision if required by policy (documented in ADR-0001).
4. `Halt` — abort pipeline; set terminal status `Halted`; do not invent outputs.
5. **Transient** errors — classification in M01; retry policy owned by the worker binary.
6. **`NotImplemented`** — fail the frame with `FailedNotImplemented`; **never** invent OCR text or detections.
7. Write `ctx.timings_ms[stage]` for every attempted stage.
8. One pipeline run per frame; config is read-only; no cross-frame mutable stage state unless a Dedup store is explicitly injected for Stage 6.

### 7.4 Backend selection

Configured in `configs/pipeline.toml` per stage (or globally):

| Backend | Meaning |
|---------|---------|
| `fixture` | Deterministic fixture implementations (CI / local spine) |
| `ai_api` | External providers via M03 (default production path) |
| `local_gpu_ext` | On-box ONNX/TensorRT extension — **`NotImplemented` until enabled** |

### 7.5 First runnable spine (fixture)

Offline path with no GPU and no network:

`image → Stage0 → Stage1(fixture) → Stage2 → Stage3(fixture OCR) → Stage4(grammar) → Stage5 → Stage6 → Stage7 → decision JSON`

This is the acceptance spine for M01 + early stage crates before AI API credentials or models exist.

---

## 8. Configuration (TOML + ENV)

### 8.1 Load order

1. Compiled / code defaults  
2. `configs/*.toml`  
3. Environment variables with prefix `TALOS__` (double-underscore nesting)  
4. Process-specific overrides (CLI), if any  

Invalid config → **fail closed** at boot (`TalosError::Config`); do not start accepting work.

### 8.2 M01-owned config files

| File | Keys (normative set) |
|------|----------------------|
| `configs/runtime.toml` | Tokio / worker thread hints, `shutdown_grace_ms`, `max_in_flight_frames` |
| `configs/decision.toml` | `auto_approve_min`, `secondary_min`, hard-fail flags |
| `configs/observability.toml` | log level, JSON vs pretty, trace sample rate, plate redaction flag |
| `configs/pipeline.toml` | stage backend selectors (`fixture` \| `ai_api` \| `local_gpu_ext`) |

### 8.3 Validation rules

- Strict mode: deny unknown keys.
- All confidence thresholds ∈ `[0.0, 1.0]`.
- Require `secondary_min < auto_approve_min`.
- Default decision thresholds: `auto_approve_min = 0.90`, `secondary_min = 0.80`.

### 8.4 Secrets

No API keys or passwords in committed TOML. Secrets for M03/M12 come from environment or a secret manager.

---

## 9. Error handling

### 9.1 Taxonomy

```text
TalosError
├── Validation      # bad input / schema — do not retry
├── Config          # boot failure — process must exit
├── NotImplemented  # missing backend — loud; countable metric
├── Transient       # timeout / 429 / unavailable — retryable
├── Permanent       # corrupt image, unsupported codec
└── Internal        # invariant violation / bug
```

### 9.2 Rules

1. M03 maps provider HTTP failures onto `Transient` or `Permanent`; M01 only defines the enum and helpers (`is_retryable()`).
2. Every error log line / span must carry `trace_id`, and when available `batch_id`, `frame_id`, `stage`.
3. An accepted batch frame must reach a `FrameTerminalStatus` before the worker considers the unit done (M10 persists; M01 requires the status field).
4. Metrics: increment `talos_errors_total{class=…,stage=…}` (naming owned with `observability` crate).

---

## 10. Tracing, health, and runtime lifecycle

### 10.1 Tracing

- Use `tracing` + `tracing-subscriber`.
- Production default: JSON logs.
- Required spans: `batch.run`, `frame.run`, `stage.process` (field `stage_id`).
- Optional later: W3C `traceparent` on HTTP APIs (M02/M12 gateways).

### 10.2 Health endpoints

| Endpoint | Semantics |
|----------|-----------|
| `GET /healthz` | Liveness — process is up |
| `GET /readyz` | Readiness — config loaded; critical deps OK when wired by other modules |

For an M01-only / fixture worker, readiness = config validated. When M10/queue are wired, readiness probes include those checks without changing M01’s endpoint contract.

### 10.3 Lifecycle

`talos_core::runtime::bootstrap()` (or equivalent `init` used under `#[tokio::main]`) must:

1. Load and validate configuration  
2. Initialize tracing  
3. Install TLS roots if required by the binary  
4. Register signal handlers  

On `SIGINT` / `SIGTERM`:

1. Stop accepting new work  
2. Drain in-flight frames up to `shutdown_grace_ms`  
3. Flush logs and exit  

---

## 11. Utilities

M01 exposes shared helpers used by intake, audit, and stages:

| Utility | Requirement |
|---------|-------------|
| SHA-256 | Content addressing for images (`image.sha256`) |
| BLAKE3 | Available for audit / provenance chaining (M10/audit consumers) |
| ID generation | ULID or UUID v7 for batch/frame/trace IDs |
| Time | UTC timestamps (`captured_at`, decision timestamps) |

---

## 12. Interfaces to M02–M12

| Module | Depends on M01 for | Must not |
|--------|-------------------|----------|
| M02 Batch & File Intake | Intake envelope types, ID helpers | Own a parallel schema |
| M03 AI Provider Gateway | `TalosError`, timeouts from config | Bypass `EvidenceKind` rules |
| M04 Image Pre-Processing | Implement Stage 0 / 2 | Change stage order |
| M05 Vision Processing | Implement Stage 1 via M03 | Emit external-DB evidence |
| M06 OCR & Plate Intelligence | Implement Stage 3 / 4 | Skip grammar when OCR is “confident” without recording evidence |
| M07 HSRP Analysis | Implement Stage 5 | Override decision thresholds |
| M08 Confidence & Decision | Outcome enums + threshold config keys | Hardcode thresholds in code paths that ignore config |
| M09 Review Management | Review action enums, decision JSON | Invent alternate terminal statuses |
| M10 Storage & Data | Serde shapes / field names | Redefine `EvidenceKind` |
| M11 Export & Reporting | DTO mirrors of decision types | |
| M12 Auth, Admin & Config | Health contract; future config-reload hooks | Embed secrets in M01 defaults |

All arrows in the hexagonal architecture are bidirectional for *data/contracts*; dependency direction for Rust crates is **inward to M01** (other modules depend on M01, not the reverse), except that M01 may define traits that other modules implement (dependency inversion).

---

## 13. Security and compliance (foundation-level)

1. **Secrets:** never commit API keys; use env / secret manager (M03, M12).
2. **Content addressing:** image bytes referenced by hash; helpers from M01.
3. **PII / plates:** treat plate text as sensitive — redact at `info` level; allow full plate at `debug` only when `observability.allow_plate_debug = true`.
4. **No elevation of VLM over grammar:** VLM evidence is additive visual observation only.

---

## 14. Testing requirements

| Layer | Requirements |
|-------|----------------|
| Unit | Config validation; serde round-trips for core types; `TalosError::is_retryable` |
| Contract | Stage registration order; `Pipeline::run` order enforcement; halt/skip behavior |
| Integration | Fixture pipeline → schema-valid decision JSON under `tests/integration/` |
| Negative | Missing backend → `NotImplemented`; invalid `decision.toml` → boot failure |

Tests must not require network, GPU, or VAHAN.

---

## 15. Documentation and ADR companions

| Document | Purpose |
|----------|---------|
| `docs/pdr/M01-foundation-and-core.md` | This PDR (normative) |
| `docs/adr/0001-pipeline-stage-contract.md` | Stage trait + Continue / SkipRemaining / Halt |
| `docs/adr/0002-evidence-kind-separation.md` | Visual vs external verification |
| `docs/adr/0003-no-local-gpu-default.md` | AI API default; GPU as extension |
| `docs/architecture/overview.md` | Hex module map M01–M12 |

ADR files may be authored when implementation begins; this PDR is authoritative for M01 behavior until superseded by a versioned PDR revision.

---

## 16. Observability conventions (M01-defined names)

Minimum metric / log field conventions (implementation in `observability` crate):

| Name | Type | Labels |
|------|------|--------|
| `talos_stage_duration_seconds` | histogram | `stage_id` |
| `talos_pipeline_frames_total` | counter | `outcome` or `terminal_status` |
| `talos_errors_total` | counter | `class`, `stage` |
| `talos_not_implemented_total` | counter | `component` |

Log / span fields: `trace_id`, `batch_id`, `frame_id`, `stage_id`, `schema_version`.

---

## 17. Revision history

| Version | Date | Notes |
|---------|------|-------|
| 1.0 | 2026-09-22 | Initial normative PDR for M01 under API-first batch architecture |

---

## 18. Open extension points (explicitly deferred)

| Extension | Status in current generation |
|-----------|------------------------------|
| Live ANPR camera ingest | Out of scope; `CameraId` remains optional metadata only |
| Local GPU / ONNX / TensorRT | `local_gpu_ext` → `NotImplemented` |
| VAHAN / transport DB | Types only (`ExternalDatabaseVerification`); no adapters |
| RabbitMQ / NATS durable queues | Not M01; scaffolded by later modules with fail-loud stubs |

These must not be faked inside M01 to appear complete.

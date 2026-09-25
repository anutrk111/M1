# PDR M03 — AI Provider Gateway

| Field | Value |
|-------|-------|
| **Module ID** | M03 |
| **Module name** | AI Provider Gateway |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) |
| **Depends on** | M01 Foundation & Core (`talos-types`, `TalosError`, config/timeouts, evidence rules) |
| **Layout** | [`Prime/Module 3/`](../../) |
| **Related docs** | [Architecture](../architecture/overview.md) · [M01 PDR](../../../Module%201/docs/pdr/M01-foundation-and-core.md) · [M02 PDR](../../../Module%202/docs/pdr/M02-batch-and-file-intake.md) |

---

## 1. Purpose

M03 is the **sole egress** for external Vision and VLM APIs used by Talos-RS. It provides:

1. Provider abstraction (HTTP REST / gRPC over TLS)
2. Timeouts, retries, and primary→fallback failover
3. Normalization of provider payloads into **M01 types**
4. Cost / token / request accounting
5. Strict evidence rules — VLM may only add `VisualObservation`

M03 does **not** orchestrate Stages 0–7, own Indian plate grammar, set decision thresholds, or call VAHAN / external registries.

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **Production vision path** | External AI APIs via M03 (`pipeline` backend `ai_api`) |
| **Local GPU** | Outside M03 (`local_gpu_ext` / ADR-0003) — M03 must not pretend to be on-box TensorRT |
| **Evidence** | Remote vision/VLM → `EvidenceKind::VisualObservation` only |
| **Secrets** | API keys via env / secret manager — never committed TOML |
| **Fail closed** | Enabled provider without credentials → `Config` or `NotImplemented`; no fake detections |
| **Layout** | `Prime/Module 3/` sibling to Module 1 / Module 2 |

---

## 3. Scope

### 3.1 In scope

| Capability | Description |
|------------|-------------|
| Provider clients | REST/gRPC TLS clients behind traits |
| Auth | Bearer / API-key headers from environment |
| Policy | Timeouts, bounded retries + jitter, failover list |
| Normalize | Map provider JSON → `Detections`, `OcrHypothesis`, `HsrpEvidence`, VLM `Evidence` |
| Cost metering | `CostEvent` per billable attempt |
| Rate limits | Map 429 → `TalosError::Transient` |
| Fixture provider | Canned responses for CI (explicit mock — not production spoofing) |
| Loud stubs | Unconfigured durable features → `NotImplemented` |

### 3.2 Out of scope

| Owner | Capability |
|-------|------------|
| M01 | Stage order, `Pipeline`, decision threshold keys |
| M02 | Folder / ZIP / upload intake |
| M04 | Local image quality / rectification algorithms |
| M05–M07 | Stage semantics; they **call** M03 |
| M06 | Deterministic Indian registration grammar (in-process) |
| M08 | When to invoke VLM; fusion thresholds |
| M10 | Persisting cost ledger long-term |
| M12 | End-user auth to Talos APIs |
| — | VAHAN / `ExternalDatabaseVerification` |

### 3.3 Non-goals

- Ad-hoc `reqwest` calls from M05–M07 bypassing M03
- VLM silently overriding grammar or clearing hard-fail flags
- Committing API keys or inventing plate text when providers are down
- Bundling on-prem vLLM as the default production path

---

## 4. Goals and success criteria

### 4.1 Goals

1. All remote vision/VLM I/O goes through M03 traits.
2. HTTP failures map to M01 `TalosError` per ADR-0007.
3. VLM outputs are additive `VisualObservation` only (ADR-0006).
4. Every billable attempt emits a `CostEvent`.
5. Missing keys for enabled providers fail closed — never fabricate detections.

### 4.2 Acceptance criteria (future implementation)

- [ ] Mock HTTP → normalized M01 `Detections` / OCR / HSRP types.
- [ ] HTTP 429 / timeout → `Transient` and retryable.
- [ ] HTTP 400 with invalid image → `Validation` or `Permanent` (documented mapping).
- [ ] `vlm_assist` evidence always `VisualObservation`; never mutates `GrammarResult`.
- [ ] Enabled provider without `API_KEY` → boot or call-time `NotImplemented`/`Config`.
- [ ] Fixture provider works offline without network.

---

## 5. Crate and app mapping (implementation binding)

**Implemented** (see [M03_IMPLEMENTATION_REPORT.md](../M03_IMPLEMENTATION_REPORT.md)). Deviations from this PDR:

- The section 6 traits and request types live in `talos_core::ports` (M01 port), re-exported by M03, so M01 stages can hold `Arc<dyn VisionGateway>` without a crate cycle.
- `VisionRequest.detection_id` was added; it is required for OCR/HSRP (G0 lineage).
- Vendor adapters are not bundled; `fixture` + generic `http_json` ship instead.

| Concern | Future location |
|---------|-----------------|
| Gateway library | `Prime/Module 3/crates/talos-ai-gateway` |
| Provider adapters | `…/src/providers/{openai,anthropic,azure,custom,fixture}.rs` |
| Config | `configs/ai_providers.toml` + `TALOS__AI__*` |
| Cost ledger | `CostRecorder` trait (in-memory for tests; M10 later) |

```text
M05/M06/M07/M08 ──► talos-ai-gateway ──► external APIs
                         │
                         ├──► talos-types (Detections, Evidence, …)
                         └──► talos-core (TalosError, tracing)
```

---

## 6. Capability APIs (normative traits)

```rust
#[async_trait]
pub trait VisionGateway: Send + Sync {
    async fn detect_vehicles_plates(
        &self,
        req: VisionRequest,
    ) -> Result<Detections, TalosError>;

    async fn run_ocr(
        &self,
        req: VisionRequest,
    ) -> Result<Vec<OcrHypothesis>, TalosError>;

    async fn analyze_hsrp(
        &self,
        req: VisionRequest,
    ) -> Result<HsrpEvidence, TalosError>;

    async fn vlm_assist(
        &self,
        req: VlmRequest,
    ) -> Result<VlmAssistResult, TalosError>;
}

pub struct VisionRequest {
    pub trace_id: TraceId,
    pub frame_id: FrameId,
    pub batch_id: BatchId,
    pub image_bytes: Option<Vec<u8>>,
    pub bytes_ref: Option<String>,
    pub content_type: String,
}

pub struct VlmRequest {
    pub vision: VisionRequest,
    pub prompt_context: String, // why assist was requested — no plate override authority
}

pub struct VlmAssistResult {
    pub evidence: Evidence, // kind MUST be VisualObservation
    pub hints: serde_json::Value, // optional structured notes for M08; non-authoritative
}
```

| Operation | Primary consumer | M01 output |
|-----------|------------------|------------|
| `detect_vehicles_plates` | M05 | `Detections` |
| `run_ocr` | M06 | `Vec<OcrHypothesis>` |
| `analyze_hsrp` | M07 | `HsrpEvidence` |
| `vlm_assist` | M08 selective path | `Evidence` + hints |

Exactly one of `image_bytes` or resolvable `bytes_ref` must be present.

---

## 7. Provider policy

### 7.1 Failover

- Per operation: ordered list `primary`, then `fallbacks[]` in config.
- On `Transient` after exhausted retries for a provider → try next fallback.
- On `Validation` / `Permanent` → do **not** failover (bad input / unsupported).

### 7.2 Retries

- Retry **only** `TalosError::Transient`.
- `max_retries` from config (default **2** additional attempts).
- Exponential backoff with jitter (bounds in config).

### 7.3 Timeouts (defaults)

| Operation | Default timeout |
|-----------|-----------------|
| Detect | 10_000 ms |
| OCR | 15_000 ms |
| HSRP | 15_000 ms |
| VLM | 30_000 ms |

### 7.4 Fixture vs production

| Mode | Behavior |
|------|----------|
| `fixture` provider | Deterministic canned JSON; no network; for CI |
| Real providers | Require API key; real HTTP |
| Missing key + enabled | `NotImplemented` or `Config` — **never** return random plates |

M03 does **not** implement `local_gpu_ext`.

---

## 8. VLM rules (critical)

1. **Who decides to call:** M08 (or review policy). M03 only executes `vlm_assist`.
2. **Evidence kind:** always `VisualObservation`; `source` = `vlm.<provider>.<model>`.
3. **Forbidden:** mutating `GrammarResult`, clearing hard-fail flags, writing `ExternalDatabaseVerification`.
4. **Hints:** optional JSON for operators/fusion — non-authoritative; fusion must still apply config thresholds and grammar.
5. **Traffic fraction:** ~5% is operational guidance; optional config `vlm.max_fraction` may enforce a soft cap later — not a hardcoded magic number in business logic.

See ADR-0006.

---

## 9. Cost tracking

### 9.1 CostEvent (normative)

```json
{
  "trace_id": "…",
  "frame_id": "…",
  "batch_id": "…",
  "provider": "openai",
  "model": "…",
  "operation": "detect_vehicles_plates",
  "status": "ok",
  "latency_ms": 120,
  "input_tokens": null,
  "output_tokens": null,
  "image_units": 1,
  "usd_estimate": null,
  "error_class": null
}
```

Emit on every billable attempt (success or failure that consumed quota).

### 9.2 Metrics

| Name | Type | Labels |
|------|------|--------|
| `talos_ai_requests_total` | counter | `provider`, `operation`, `status` |
| `talos_ai_latency_seconds` | histogram | `provider`, `operation` |
| `talos_ai_cost_usd_total` | counter | `provider`, `model` |
| `talos_ai_fallbacks_total` | counter | `operation`, `from_provider`, `to_provider` |

Persistence of cost history is **M10**; M03 defines the event + in-memory recorder for tests.

---

## 10. Error mapping (summary)

Normative detail in ADR-0007:

| Provider / HTTP condition | `TalosError` |
|---------------------------|--------------|
| Timeout, 429, 502, 503, 504 | `Transient` |
| 401, 403 (bad credentials) | `Config` or `Permanent` (call-site policy: treat as non-retryable) |
| 400 invalid image / schema | `Validation` |
| 404 model not found | `Permanent` |
| Unconfigured provider | `NotImplemented` |

---

## 11. Configuration

**Load order:** defaults → `configs/ai_providers.toml` → `TALOS__AI__*` env.

### 11.1 Example (no secrets)

See committed [`configs/ai_providers.toml`](../../configs/ai_providers.toml).

Secrets:

```text
TALOS__AI__OPENAI__API_KEY=…
TALOS__AI__ANTHROPIC__API_KEY=…
TALOS__AI__AZURE__API_KEY=…
```

---

## 12. Security and compliance

1. Never log raw API keys or full `Authorization` headers.
2. Redact provider response bodies at `info` if they echo plate text unless `allow_plate_debug` (M01 observability).
3. TLS required for all external providers.
4. No VAHAN or registry calls from M03.

---

## 13. Testing requirements (future implementation)

| Layer | What |
|-------|------|
| Unit | Error mapping table; cost event fields; VLM evidence kind guard |
| Contract | Trait object + fixture provider → M01 serde round-trip |
| Integration | Wiremock/httpmock → detect/OCR paths |
| Negative | Missing key; 429 retry then fallback; VLM cannot set ExternalDatabaseVerification |

No real paid API calls required in CI.

---

## 14. Documentation and ADR companions

| Document | Purpose |
|----------|---------|
| `docs/pdr/M03-ai-provider-gateway.md` | This PDR |
| `docs/adr/0006-vlm-evidence-additive-only.md` | VLM additive visual evidence only |
| `docs/adr/0007-provider-error-mapping.md` | HTTP → `TalosError` |

---

## 15. Interfaces to other modules

| Module | Relationship |
|--------|----------------|
| M01 | Types, errors, tracing |
| M02 | Supplies images via envelopes; M03 does not intake files |
| M04 | May prepare crops; M03 still receives bytes |
| M05–M07 | Call gateway ops for stages 1/3/5 when backend=`ai_api` |
| M08 | Invokes `vlm_assist` selectively; owns thresholds |
| M09 | May display VLM evidence in review UI |
| M10 | Future cost ledger persistence |
| M12 | Does not replace provider API keys with user JWT |

---

## 16. Revision history

| Version | Date | Notes |
|---------|------|-------|
| 1.0 | 2026-09-22 | Initial normative PDR for M03 AI Provider Gateway |

---

## 17. Open extension points (deferred)

| Extension | Status |
|-----------|--------|
| Streaming token providers | Out of scope |
| On-prem vLLM as default | Extension only |
| Automatic cheapest-provider routing | Non-goal |
| Local GPU / TensorRT | Not M03 |
| Cross-cloud identity federation | Deferred |

Do not fake these inside M03 to appear complete.

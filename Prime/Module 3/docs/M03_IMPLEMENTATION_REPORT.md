# M03 Implementation Report — AI Provider Gateway

| Item | Value |
|------|-------|
| Branch | `impl/ai-gateway` (from `integration/m01-m03` after Gate G0) |
| Crate | `Prime/Module 3/crates/talos-ai-gateway` |
| Port | `talos_core::ports::VisionGateway` (M01-owned; M03 is the adapter) |
| Config | [`configs/ai_providers.toml`](../configs/ai_providers.toml) + `TALOS__AI__*` env |

## Architecture

```text
M05/M06/M07/M08 ──► talos_core::VisionGateway (port)
                        ▲
                        │ implements
               talos-ai-gateway::AiGateway
                        │ per operation route: primary → fallbacks
                        ▼
      [circuit breaker] → [token bucket] → [timeout] → Provider::call
                        │                                  │
                  bounded retry (Transient only)     fixture | http_json | custom
                        │
                 normalize → frozen M01 types (G0)
                        │
                 CostEvent + metrics + provider health
```

The PDR placed the traits in M03. They live in `talos-core` instead because `talos-core` stages must accept a gateway trait object, and `talos-core` cannot depend on M03 without a dependency cycle. M03 re-exports them.

## Public API

| Item | Purpose |
|------|---------|
| `AiGateway::from_config(cfg, cost)` / `AiGateway::builder(cfg).with_provider(..).with_cost_recorder(..).build()` | Boot. Fails closed with `Config`. |
| `VisionGateway` (`detect_vehicles_plates`, `run_ocr`, `analyze_hsrp`, `vlm_assist`, `run_ocr_consensus`) | Capability port |
| `Provider` trait, `ProviderRequest`, `ProviderResponse` | Adapter SPI |
| `providers::{FixtureProvider, HttpJsonProvider, ApiKey}` | Built-in adapters |
| `CostRecorder`, `InMemoryCostRecorder`, `NoopCostRecorder` | Accounting |
| `AiGateway::health() -> Vec<ProviderHealth>` | Breaker state, ok/error totals, last error class |
| `AiGateway::metrics() -> MetricsSnapshot` | `talos_ai_requests_total`, `talos_ai_latency_seconds`, `talos_ai_cost_usd_total`, `talos_ai_fallbacks_total` |
| `load_ai_config(dir)` | defaults → TOML → env (keys ending `api_key` are never read into config) |

## Request contract

`VisionRequest` gained `detection_id: Option<DetectionId>`. `run_ocr` and `analyze_hsrp` require it, and return `Validation` *before* any provider call when it is missing. This follows the frozen G0 rule that OCR/HSRP artifacts carry `DetectionId`.

Other request rules:

- Exactly one of `image_bytes` / `bytes_ref` must be set.
- Content type must be `image/jpeg` or `image/png`.
- `file://` refs are read only from `[ai].local_artifact_roots` (canonicalized; see the PR #6 remediation addendum), up to `max_request_image_bytes`. With no roots configured every `file://` ref is a `Config` error.
- Other schemes return `NotImplemented` (object-store resolution is M10).

## Policy semantics

| Concern | Behavior |
|---------|----------|
| Timeout | Per operation (`[ai.timeouts_ms]`), covering the rate-limit wait plus the call. Expiry maps to `Transient`. |
| Retry | Only `Transient`. `max_retries` extra attempts, with exponential backoff `base * 2^n`, capped at `max_delay_ms`, jitter factor in [0.5, 1.0]. |
| Failover | The next provider is tried only after `Transient` retries are exhausted, or when the circuit is open. `Validation`, `Permanent`, `Config` and `NotImplemented` return immediately (ADR-0007). |
| Circuit breaker | Per provider. Opens after `failure_threshold` consecutive `Transient` failures. After `open_ms` it goes half-open (`half_open_max_calls` probes). A probe success closes it; a probe failure reopens it. Non-transient outcomes are neutral. An open circuit short-circuits without calling the provider. |
| Rate limit | Per-provider token bucket (`requests_per_second`, `burst`, optional per-provider override). It waits for a token, bounded by the operation timeout. |
| Tie-breaker ("Emperor") | `run_ocr_consensus`: primary route, then the configured secondary. If the normalized top texts disagree, the arbiter is called. All hypotheses are returned with their `ProviderRef`; nothing overrides M06 grammar (ADR-0006/0019). A secondary/arbiter failure leaves its opinion `None`, never a fabricated agreement. |
| Fixture vs live | `default_mode = "live"` rejects any fixture-kind provider in a route at boot. |
| Secrets | An enabled `http_json` provider without a non-empty `TALOS__AI__<NAME>__API_KEY` (or `api_key_env`) is a boot-time `Config` error. `ApiKey` `Debug` output is redacted. `base_url` is parsed as a URL: `https://` is required, plain `http://` only for an exact loopback host (`localhost`, `127.0.0.0/8`, `::1`); userinfo, query, fragment and malformed URLs are `Config` errors. |

## HTTP error mapping (ADR-0007)

| HTTP | `TalosError` |
|------|--------------|
| 408, 429, 500, 502, 503, 504; timeout / connect / transport | `Transient` |
| 400, 413, 415, 422 | `Validation` |
| 401, 403 | `Config` (not retried, not billed) |
| other (e.g. 404) | `Permanent` |
| malformed 2xx body | `Permanent` |

HTTP 500 is treated as `Transient`. ADR-0007 lists only 502/503/504, so this is a documented extension: a generic 500 from inference backends is usually overload. Error messages carry only the status code, never the provider body, because it may echo plate text.

## Normalized wire format (`http_json` and fixture)

`POST {base_url}/v1/{detect_vehicles_plates|run_ocr|analyze_hsrp|vlm_assist}`, bearer auth, request body:

```json
{
  "trace_id": "...",
  "frame_id": "...",
  "batch_id": "...",
  "detection_id": "...",
  "model": "...",
  "content_type": "image/jpeg",
  "image_base64": "...",
  "prompt_context": null
}
```

Response body is `{ "result": <wire>, "usage": { "input_tokens", "output_tokens", "usd_estimate" } }`, where `<wire>` depends on the operation:

| Operation | `<wire>` shape | M01 output |
|-----------|----------------|------------|
| detect | `{ plates: [{label?, score, bbox:{x_min,y_min,x_max,y_max}}], vehicles: [...] }` | `Detections` (no `DetectionId`: M05 mints) |
| ocr | `{ hypotheses: [{text, confidence, char_confidences?}] }` | `Vec<OcrHypothesis>` bound to the request `DetectionId` |
| hsrp | `{ score, ind_mark, hologram, geometry: "OBSERVED" \| "NOT_OBSERVED" \| "NOT_OBSERVABLE", notes? }` | `HsrpEvidence` |
| vlm | `{ confidence, observation?, hints?, evidence_kind? }` | `Evidence` (`VisualObservation`, `source = vlm.<provider>.<model>`) + non-authoritative hints |

The gateway rejects the following as `Permanent`:

- an out-of-range or inverted bbox, or a pixel-shaped bbox;
- scores or confidences outside [0,1];
- boolean HSRP cues;
- empty OCR text;
- any VLM `evidence_kind` other than `visual_observation`.

Vendor-specific adapters (OpenAI, Anthropic, Azure, ...) plug in through the `Provider` trait or a `http_json` shim. None are bundled.

## Cost accounting

A `CostEvent` (frozen G0 contract) is emitted on every attempt that reached a provider and may have consumed quota: success, `Transient`, `Validation`, `Permanent`. It carries provider, model, operation, status, latency, tokens, `image_units = 1`, `usd_estimate` and `error_class`. Local short-circuits (open circuit, rate-limit timeout) and `Config`/`NotImplemented` are not billed. Persistence is M10.

## Tests (35)

- **Unit (15):**
  - config loads the committed TOML;
  - API key env naming;
  - breaker open/half-open/close and reopen;
  - failure reset;
  - token bucket wait;
  - bounded jittered backoff;
  - ADR-0007 status table;
  - `ApiKey` redaction;
  - TLS enforcement;
  - normalization guards (bbox/score, pixel bbox, missing ids, boolean HSRP, VLM external evidence, OCR lineage and empty text).
- **Integration (20, `tests/gateway.rs`):**
  - fixture round-trip to M01 types with cost events;
  - 429 retry then failover (wiremock), with cost/metrics/usage;
  - 503 then success;
  - timeout maps to `Transient`;
  - 400 maps to `Validation` without failover and without leaking the body;
  - 401 maps to `Config`, not retried, not billed;
  - malformed geometry maps to `Permanent`;
  - VLM non-visual evidence rejected over HTTP;
  - VLM disabled maps to `NotImplemented`;
  - missing key fails closed;
  - live mode rejects fixture routes and unknown providers;
  - lineage is required before any call;
  - request validation;
  - `file://` resolution;
  - breaker open and half-open recovery;
  - an open circuit fails over;
  - rate-limit spacing;
  - tie-breaker (disagree calls the arbiter; agree does not);
  - tie-breaker config validation;
  - trait-object use.

No real network or paid APIs; wiremock binds loopback only.

## PR #6 remediation addendum

Two boundary fixes from the PR #6 review. The test counts above describe the original lane; after remediation the crate has 18 unit tests and 24 gateway tests.

| Finding | Cause | Fix |
|---------|-------|-----|
| Plain-HTTP loopback exception | `base_url.starts_with("http://localhost")` also matched `http://localhost.evil.example`, so the bearer key could go over plaintext to a non-loopback host. | `providers::http_json::validate_base_url` parses with the `url` crate and checks the parsed host (`Domain("localhost")`, IPv4 `is_loopback`, IPv6 `is_loopback`). Userinfo (`localhost@evil.example`), query / fragment and malformed URLs are `Config`. Errors never echo the URL. |
| Arbitrary `file://` reads | `resolve_image` read any absolute path, so a forged `VisionRequest` could upload e.g. `/etc/passwd` to a provider. | New `[ai].local_artifact_roots` (default empty = refuse). The path must be absolute; it and each root are canonicalized (resolving `..` and symlinks) and compared by whole path components, so sibling-prefix paths (`/stage-evil`) and symlink escapes fail. Outside a root or not a regular file: `Validation`, with no provider call and no cost event. Reads are capped at `max_request_image_bytes + 1`. |

Deployments set `local_artifact_roots` to the M02 `staging_root` (TOML or `TALOS__AI__LOCAL_ARTIFACT_ROOTS`). Residual risk: a process that can write inside an authorized root could swap a path component between canonicalization and open; the roots should be writable only by Talos.

New tests: `exact_loopback_http_accepted`, `loopback_lookalikes_and_malformed_rejected_as_config`, `config_errors_do_not_echo_the_url` (unit); `file_bytes_ref_inside_authorized_root_is_resolved`, `file_bytes_ref_refused_without_configured_roots`, `file_bytes_ref_outside_root_rejected_without_egress` (absolute outside, `..`, prefix sibling, root itself, `/etc/passwd`, relative), `file_bytes_ref_symlink_escape_rejected` (file and directory symlinks), `file_bytes_ref_over_size_cap_rejected` (integration). The earlier `file_bytes_ref_is_resolved` test became the authorized-root test.

## Deferred

- Vendor-specific adapters and real credentials: deployment.
- Object-store `bytes_ref` resolution: M10.
- Cost ledger persistence: M10.
- Prometheus exporter binding: observability backend (snapshot API provided).
- `vlm.max_fraction` enforcement: M08 policy.
- gRPC transport.

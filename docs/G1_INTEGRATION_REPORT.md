# Gate G1 — M01 + M02 + M03 Integration Report

| Item | Value |
|------|-------|
| Integration branch | `integration/m01-m03` |
| Lanes merged | [#1](https://github.com/anutrk111/M1/pull/1) G0 contract freeze, [#2](https://github.com/anutrk111/M1/pull/2) M03 AI gateway, [#3](https://github.com/anutrk111/M1/pull/3) M01 gateway wiring, [#4](https://github.com/anutrk111/M1/pull/4) M02 intake hardening |
| G1 branch | `impl/g1-integration` |
| Gate test | `Prime/Module 1/crates/talos-integration/tests/g1_intake_gateway.rs` |

## What G1 proves

The G1 crate is test-only (`publish = false`). It drives the real M02 intake into the real M03 `AiGateway`, using the fixture provider, through the M01 `VisionGateway` port. It uses no network, no paid APIs, and no M04–M12 business logic.

| Test | Asserts |
|------|---------|
| `folder_intake_to_gateway_detect_preserves_contracts_and_lineage` | Folder intake reconciles: 2 accepted, 1 duplicate, 1 ignored. Envelopes carry `INTAKE_SCHEMA_VERSION` and round-trip through JSON. The envelope SHA-256 equals both the original bytes and the staged copy, and originals are unchanged. `VisionRequest::for_frame` keeps `batch_id` / `frame_id`. Gateway detections have normalized, valid bounding boxes, a provider ref and `detection_id = None` (M05 mints). There is exactly one `CostEvent` per call with matching trace / frame / batch ids, and the metrics count 2 successful detect requests. |
| `upload_intake_plate_ocr_and_hsrp_are_bound_to_detection_id` | Upload intake, then detect, then a plate request. Every OCR consensus hypothesis and the HSRP evidence carry the request's `DetectionId`. `OcrResult::validate_lineage` passes. HSRP cues serialize as the ternary `OBSERVED` / `NOT_OBSERVED` / `NOT_OBSERVABLE`. Cost events keep the frame lineage. |
| `ocr_and_hsrp_without_detection_id_are_rejected_before_egress` | OCR and HSRP on a whole-frame request (no `DetectionId`) fail with `Validation`, and no provider call or cost event occurs. |
| `fixture_pipeline_accepts_real_m02_envelope` | An all-fixture Stage 0–7 pipeline succeeds on an M02 envelope. The decision JSON carries `DECISION_SCHEMA_VERSION` and the same batch / frame ids. |
| `ai_api_pipeline_with_real_gateway_fails_closed_until_m04_m05_land` | With the real gateway on stage 1, the frame ends `FailedValidation` (no `DetectionId` minting yet). On stage 3 it ends `FailedNotImplemented` (the fixture rectification emits `fixture://` crops, which the gateway refuses). Neither produces a fused decision. |
| `duplicate_upload_is_recorded_not_sent_to_gateway` | A byte-identical upload is recorded as `SkippedDuplicate` pointing to the original `FrameId`, and only one envelope reaches the sink. |

## Stand-ins and known gaps (by design)

- **DetectionId minting (M05) and rectified crops (M04) are not implemented.** The plate-path test mints a stand-in id and uses the staged original as the crop. It proves the port carries the lineage; it does not show real rectification.
- **An all-`ai_api` pipeline does not produce decisions yet.** It fails closed, as the last row above shows. That is the expected outcome until M04 and M05 replace the fixture stages in Wave 2.
- **`bytes_ref` resolution covers `file://` only.** Object-store references return `NotImplemented` until M10.
- The M03 deviations listed in `Prime/Module 3/docs/M03_IMPLEMENTATION_REPORT.md` still apply. HTTP 408 and 500 are treated as transient, and `vlm.max_fraction` enforcement belongs to M08.

## Verification (local, `Prime/`)

| Command | Result |
|---------|--------|
| `cargo metadata --no-deps --format-version 1` | ok |
| `cargo fmt --all -- --check` | ok |
| `cargo check --workspace --all-targets` | ok |
| `cargo clippy --workspace --all-targets -- -D warnings` | ok |
| `cargo test --workspace` | 131 passed, 0 failed |
| `git diff --check` | ok |

Per-suite counts:

| Suite | Tests |
|-------|-------|
| `talos-types` contracts | 18 |
| `talos-config` unit | 4 |
| `talos-core` unit | 6 |
| `talos-core` pipeline fixture | 3 |
| `talos-core` backend wiring | 5 |
| `talos-intake` unit | 20 |
| `talos-intake` integration | 21 |
| `talos-intake` hardening | 13 |
| `talos-ai-gateway` unit | 15 |
| `talos-ai-gateway` gateway | 20 |
| G1 | 6 |

Remote CI runs on the PR `integration/m01-m03` → `main`.

## Next

The PR `integration/m01-m03` → `main` is ready for review and should not be merged without it. After merge, Wave 2 (M04, M05) starts from `main` on `impl/*` branches into `integration/m04-m07`, against the frozen G0 contracts.

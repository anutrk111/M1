# PDR M04 — Image Pre-Processing

| Field | Value |
|-------|-------|
| **Module ID** | M04 |
| **Module name** | Image Pre-Processing |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) — APPROVED with multi-plate / provenance refinements |
| **Depends on** | M01 Stage trait / FrameContext / TalosError; Stage 1 detections from M05 (via context); images from M02 |
| **Layout** | [`Prime/Module 4/`](../../) |
| **Related** | [Architecture](../architecture/overview.md) · [M01 PDR](../../../Module%201/docs/pdr/M01-foundation-and-core.md) · [M03 PDR](../../../Module%203/docs/pdr/M03-ai-provider-gateway.md) |

---

## 1. Purpose

M04 **prepares pixels**; it does **not** decide what those pixels mean.

| Stage | Name | M04 duty |
|-------|------|----------|
| **0** | Image Quality Assessment | Measure decode integrity, resolution, blur, exposure, contrast → evidence |
| **2** | Plate Rectification & Enhancement | After Stage 1 detections: crop, deskew, enhance, API-prepare up to `top_k` plates |

**Audit principle (mandatory):**

> Original evidence never changes; enhancement is always a derived artifact.

Review Officers must be able to inspect **original frame**, **original plate crop**, and **enhanced / API-prepared** images separately.

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **Original immutable** | Never overwrite/recompress M02 original bytes; M02 SHA-256 remains authoritative for the frame |
| **Stage 0** | Evidence, not final decision; soft fail → continue |
| **Stage 2** | Multi-plate capable; default `top_k = 1`; contract is `Vec` / `RectificationResult` |
| **Three representations** | `OriginalFrame`, `PlateCrop`, `ApiPreparedImage` — do not mix |
| **No OCR invention** | No character infer/reconstruct |
| **No-plate** | Typed `NoPlateDetected`, not an M04 software error |
| **M03 preference** | Plate-only ops should receive crop / api-prepared, not full vehicle frame when possible |

---

## 3. Scope

### 3.1 In scope

- Stage 0 quality metrics + `Accept` / `Degraded` / `Unusable`
- Stage 2 bbox validate, padding, crop, perspective/deskew, enhance, API resize/compress
- Derived SHA-256 + `TransformProvenance`
- Fixture/CPU heuristics for CI (no network/GPU required)
- Document M01 type extensions required by this contract

### 3.2 Out of scope

| Owner | Capability |
|-------|------------|
| M02 | Intake / authoritative original hash |
| M03 | External Vision/VLM HTTP |
| M05 | Plate/vehicle detection |
| M06 | OCR + Indian grammar |
| M07 | HSRP evidence semantics |
| M08 | Final Auto/Secondary/Review decision |
| M10 | Durable object storage beyond staging refs |

### 3.3 Non-goals

- Treating blur/darkness alone as final rejection
- Single `plates[0]`-only architecture (default top_k may be 1, contract must be N-ready)
- Mutating original frame bytes
- Calling AI APIs from M04

---

## 4. M01 type extensions (normative — implement at build time)

Current Module 1 `RectifiedPlate` / `ImageQualityReport` are **superseded** by this PDR for future builds. Until types are updated in `talos-types`, this document is authoritative.

### 4.1 Quality

```rust
pub enum ImageQualityClass {
    Accept,
    Degraded,
    Unusable,
}

pub struct ImageQualityReport {
    pub class: ImageQualityClass,
    pub score: f32,
    pub blur_score: f32,
    pub exposure_ok: bool,
    pub contrast_ok: bool,
    pub width: u32,
    pub height: u32,
    pub decode_ok: bool,
    pub notes: Vec<String>,
}
```

### 4.2 Image representation kinds

```rust
pub enum ImageRepresentationKind {
    OriginalFrame,    // M02 authoritative
    PlateCrop,        // lossless-ish crop of original (no API recompress intent)
    ApiPreparedImage, // resized/recompressed for M03 cost/privacy
}
```

### 4.3 Rectification

```rust
pub struct DetectionId(pub String); // stable id for a Stage 1 plate detection within the frame

pub struct TransformProvenance {
    pub steps: Vec<TransformStep>, // name + parameters JSON
}

pub struct TransformStep {
    pub name: String,       // e.g. "clahe", "unsharp", "deskew"
    pub parameters: serde_json::Value,
}

pub struct RectifiedPlate {
    pub detection_id: DetectionId,
    pub source_bbox: BoundingBox,

    pub original_crop_ref: String,           // BytesRef / file://…
    pub enhanced_crop_ref: Option<String>,
    pub api_prepared_ref: Option<String>,

    pub crop_sha256: String,
    pub enhanced_sha256: Option<String>,
    pub api_prepared_sha256: Option<String>,

    pub width: u32,
    pub height: u32,

    pub transform: TransformProvenance,
}

pub enum RectificationResult {
    Plates(Vec<RectifiedPlate>),
    NoPlateDetected,
}
```

`FrameContext` must carry:

- `quality: Option<ImageQualityReport>`
- `rectification: Option<RectificationResult>`  
  (replacing a single `Option<RectifiedPlate>`)

---

## 5. Goals and acceptance (future implementation)

- [ ] Original envelope `image.sha256` unchanged after Stages 0 and 2.
- [ ] Soft quality → `Degraded` + `Continue`; Unusable decode → `Halt` only.
- [ ] `top_k` plates produced when detections ≥ top_k; architecture supports N.
- [ ] Zero Stage 1 plates → `NoPlateDetected` + `Continue` (not `TalosError::Validation` from M04).
- [ ] Provenance + derived hashes when config flags enabled.
- [ ] Offline fixture path; no network/GPU for default CPU path.

---

## 6. Crate mapping (build later)

| Concern | Path |
|---------|------|
| Library | `Prime/Module 4/crates/talos-preprocess` |
| Stage 0 | `ImageQualityStage` |
| Stage 2 | `PlateRectifyStage` |
| Config | `configs/preprocess.toml` + `TALOS__PREPROCESS__*` |

---

## 7. Stage 0 — Image Quality Assessment

### 7.1 Inputs / outputs

- **In:** original frame bytes (read-only) from M02 envelope.
- **Out:** `ImageQualityReport` + optional `VisualObservation` evidence (`source = preprocess.s0`).

### 7.2 Quality class mapping (ADR-0008)

| Class | Typical causes | Pipeline |
|-------|----------------|----------|
| `Accept` | Within thresholds | `Continue` |
| `Degraded` | Soft blur/dark/low-res warn | `Continue` + evidence (**not** final reject) |
| `Unusable` | Undecodable, corrupt, unsupported codec | `Halt` |

Default: `on_soft_fail = "continue"`. Blur/darkness alone must not halt.

### 7.3 Checks

Decode integrity, width/height vs `min_width`/`min_height`, blur score, exposure, contrast, composite `score`.

---

## 8. Stage 2 — Rectification & Enhancement

### 8.1 Inputs / outputs

- **In:** Stage 1 `Detections.plates` (0..N), original frame (read-only).
- **Out:** `RectificationResult::Plates(Vec<…>)` or `NoPlateDetected`.

### 8.2 Plate selection

1. Sort plates by detection score descending.
2. Take up to `rectification.top_k` (default **1**).
3. Assign/propagate `detection_id` per plate.
4. Never invent plates when list is empty → `NoPlateDetected`.

### 8.3 Per-plate pipeline (order fixed)

1. Validate bbox (in-bounds, min size)
2. Add safe padding (`crop_padding_percent`)
3. **Original plate crop** → `original_crop_ref` + `crop_sha256`
4. Perspective / deskew (if enabled)
5. Contrast / CLAHE / denoise / unsharp (if enabled) → `enhanced_crop_ref` + `enhanced_sha256`
6. API-size preparation (`max_edge_px`, `jpeg_quality`) → `api_prepared_ref` + optional hash
7. Record `TransformProvenance`

### 8.4 No-plate outcome

If Stage 1 legitimately returns 0 plates:

- Emit `RectificationResult::NoPlateDetected`
- Return `StageStatus::Continue`
- Do **not** raise M04 `Validation` solely for empty detections  
  (M08 / `hard_fail_on_no_plate` owns the decision)

Software failures (I/O, panic-class bugs, invalid config) remain `TalosError` — distinct from detection absence.

### 8.5 Evidence

Optional visual evidence `preprocess.s2` summarizing plate count / transform names — never OCR text.

---

## 9. Interaction with M03

| Downstream need | Preferred bytes sent via M03 |
|-----------------|------------------------------|
| OCR / HSRP on plate | `api_prepared_ref` or `enhanced_crop_ref`, else `original_crop_ref` |
| Full-scene vision (rare for plate ops) | Original frame only when required |

Privacy/cost: avoid sending full vehicle frame when only the plate is required.

---

## 10. Configuration

See committed [`configs/preprocess.toml`](../../configs/preprocess.toml).

Defaults are **starting points**, not accuracy claims; tune after fixture testing.

Env overlay: `TALOS__PREPROCESS__*` (aligned with M01 config style).

---

## 11. Observability

| Metric | Labels |
|--------|--------|
| `talos_preprocess_quality_total` | `class` = accept\|degraded\|unusable |
| `talos_preprocess_rectify_total` | `status` = plates\|no_plate\|error |
| `talos_preprocess_plates_emitted` | — |
| `talos_preprocess_provenance_hashes_total` | `artifact` = crop\|enhanced\|api |

---

## 12. Testing (future implementation)

| Case | Expect |
|------|--------|
| Soft blur | `Degraded` + Continue; original hash unchanged |
| Corrupt bytes | `Unusable` + Halt |
| 0 plates | `NoPlateDetected` + Continue |
| 3 plates, top_k=2 | Exactly 2 `RectifiedPlate`s |
| Enhancement on | provenance steps + derived hashes |
| OCR invention | Forbidden — no text fields from M04 |

---

## 13. ADRs

| ADR | Topic |
|-----|-------|
| [0008](../adr/0008-stage0-quality-gates.md) | Accept / Degraded / Unusable |
| [0009](../adr/0009-rectify-after-detection.md) | After Stage 1; multi-plate; provenance; NoPlateDetected |

---

## 14. Interfaces

| Module | Relationship |
|--------|----------------|
| M01 | Orchestrates stages; will host extended types |
| M02 | Supplies immutable original |
| M03 | Consumes crops for plate ops |
| M05 | Supplies 0..N detections before Stage 2 |
| M06/M07 | Consume rectified plates via M03 |
| M08 | Interprets quality + NoPlateDetected + OCR/HSRP |
| M09 | Shows original vs derived artifacts in review |

---

## 15. Revision history

| Version | Date | Notes |
|---------|------|-------|
| 1.0 | 2026-09-22 | Initial approved PDR with multi-plate + provenance refinements |

---

## 16. Extensions deferred

GPU acceleration; learned quality models via M03; production tuning of `top_k > 1`. Do not fake these in M04.

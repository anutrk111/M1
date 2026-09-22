# PDR M07 — HSRP Analysis

| Field | Value |
|-------|-------|
| **Module ID** | M07 |
| **Module name** | HSRP Analysis |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) — APPROVED / DESIGN LOCKED |
| **Owns** | M01 **Stage 5** — HSRP visual evidence |
| **Depends on** | M01 types/errors; M03 `analyze_hsrp`; M04 RectifiedPlate; M05 DetectionId |
| **Layout** | [`Prime/Module 7/`](../../) |
| **Related** | [Architecture](../architecture/overview.md) · [M03](../../../Module%203/docs/pdr/M03-ai-provider-gateway.md) · [M06](../../../Module%206/docs/pdr/M06-ocr-plate-intelligence.md) · [M08](../../../Module%208/docs/pdr/M08-confidence-decision.md) |

---

## 1. Purpose

M07 records **visual observations** related to HSRP cues on a RectifiedPlate. It does **not** declare legal compliance or illegality.

**Lineage invariant (mandatory):**

> Every HSRP observation MUST retain the originating Talos `DetectionId`.

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **Remote path** | Via **M03 only** (`analyze_hsrp`); fixture for CI |
| **Authority** | Observations only — **not** legal/compliance verdict |
| **Output shape** | Individual observations + confidence + source artifact + DetectionId |
| **Observability** | `Observed` / `NotObserved` / `NotObservable`\|`Unknown` — never collapse Unknown → false |
| **Evidence** | `VisualObservation` only (`hsrp.s5`) |
| **No plate** | No HSRP call; empty evidence for that id |

---

## 3. Scope

### 3.1 In scope

- Stage 5 analysis via M03 for each RectifiedPlate
- Cue observations: IND mark visibility, hologram-like cues, plate geometry, color/layout/visual pattern hints
- Provenance and DetectionId binding
- Ternary observability semantics

### 3.2 Out of scope

| Owner | Capability |
|-------|------------|
| M03 | HTTP, retries, cost |
| M06 | OCR / grammar |
| M08 | Fusion, legal-ish outcomes, thresholds |
| VAHAN | ExternalDatabaseVerification |
| Human review | Final compliance adjudication |

### 3.3 Forbidden outputs

- “Legal HSRP” / “illegal plate” as M07 authority
- Invented registration numbers
- `AutoApproved` / other decision outcomes
- Boolean coercion of unobservable cues to `false`

---

## 4. Observation model (normative)

```rust
pub struct DetectionId(pub String);

pub struct ProviderRef {
    pub name: String,
    pub model: Option<String>,
}

pub enum HsrpSourceArtifact {
    ApiPrepared,
    Enhanced,
    OriginalCrop,
    Other(String),
}

/// Ternary: not visible ≠ not present.
pub enum Observability {
    Observed,
    NotObserved,
    NotObservable, // alias semantics: Unknown / cannot claim
}

pub enum HsrpCueKind {
    IndMark,
    HologramLike,
    PlateGeometry,
    ColorLayout,
    VisualPattern,
    Other(String),
}

pub struct HsrpObservation {
    pub detection_id: DetectionId,
    pub source_artifact: HsrpSourceArtifact,
    pub cue: HsrpCueKind,
    pub observability: Observability,
    pub confidence: Option<f32>,
    pub provider: ProviderRef,
    pub detail: Option<String>,
}

pub struct HsrpEvidenceBundle {
    pub detection_id: DetectionId,
    pub observations: Vec<HsrpObservation>,
    pub overall_score: Option<f32>, // optional aggregate; NOT a legal verdict
}
```

---

## 5. Stage 5 behavior (locked)

1. Input: RectifiedPlate with same `DetectionId` as M06 path.
2. Prefer `api_prepared` crop (`prefer_image`).
3. Call M03 `analyze_hsrp`; map to `HsrpObservation[]`.
4. If a cue cannot be assessed from the image → `NotObservable`/`Unknown`, not `false`/`NotObserved`.
5. Soft Degraded quality does not alone block HSRP; Unusable already halted earlier.
6. Empty / NoPlateDetected → skip call; empty bundle.

---

## 6. Evidence

| Kind | Code | Notes |
|------|------|-------|
| VisualObservation | `hsrp.s5` | Per-cue observations + provenance |

Never ExternalDatabaseVerification. Never imply VAHAN confirmation.

---

## 7. Observability metrics

- Stage 5 latency
- Observation counts by cue and Observability
- Rate of `NotObservable` (signal of crop/quality limits)

---

## 8. Acceptance criteria

- [ ] Every observation carries `DetectionId` + source artifact + provenance.
- [ ] No M07 legal/compliance verdict fields.
- [ ] Unknown/NotObservable distinct from NotObserved.
- [ ] Boolean collapse disabled by default.
- [ ] Fixture path works offline.
- [ ] Docs only under `Prime/Module 7/`.

---

## 9. Build mapping (later)

| Crate | Role |
|-------|------|
| `talos-hsrp` | Stage 5 |

No Rust in this docs step.

# PDR M06 — OCR & Plate Intelligence

| Field | Value |
|-------|-------|
| **Module ID** | M06 |
| **Module name** | OCR & Plate Intelligence |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) — APPROVED / DESIGN LOCKED |
| **Owns** | M01 **Stage 3** (OCR) and **Stage 4** (Indian registration grammar) |
| **Depends on** | M01 types/errors; M03 `run_ocr`; M04 RectifiedPlate; M05 DetectionId |
| **Layout** | [`Prime/Module 6/`](../../) |
| **Related** | [Architecture](../architecture/overview.md) · [M03](../../../Module%203/docs/pdr/M03-ai-provider-gateway.md) · [M04](../../../Module%204/docs/pdr/M04-image-pre-processing.md) · [M05](../../../Module%205/docs/pdr/M05-vision-processing.md) |

---

## 1. Purpose

M06 reads characters from a **RectifiedPlate** and validates/normalizes them against Indian registration grammar using **OCR evidence**. It does **not** decide HSRP legality or emit final pipeline outcomes.

**Lineage invariant (mandatory):**

> `OriginalFrame → DetectionId → RectifiedPlate → OCR/Grammar → Decision`

Every OCR hypothesis and grammar result MUST retain the originating Talos `DetectionId`.

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **Remote OCR** | Via **M03 only** (`run_ocr`); fixture backend for CI |
| **Hypothesis shape** | Structured object — **not** a plain string |
| **Grammar** | Local, deterministic; evidence-supported corrections only |
| **Never bypass** | High OCR confidence never skips grammar |
| **No invent** | Grammar does not create a new plate identity |
| **Evidence** | `VisualObservation` only (`ocr.s3`, grammar validation notes) |
| **NoPlateDetected** | Skip OCR/grammar for that frame path; do not invent text |

---

## 3. Scope

### 3.1 In scope

- Stage 3 OCR via M03 for each RectifiedPlate (`DetectionId`)
- Structured hypotheses with provenance and optional character evidence
- Stage 4 Indian registration grammar → `VALID` / `KNOWN_INVALID` / `UNKNOWN_TO_REGISTRY`
- VisualObservation evidence for OCR and grammar

### 3.2 Out of scope

| Owner | Capability |
|-------|------------|
| M03 | HTTP clients, retries, cost for OCR |
| M04 | Quality, rectify/enhance crops |
| M05 | Detection / DetectionId minting |
| M07 | HSRP observations |
| M08 | Dedup, fusion, final outcomes |
| VAHAN / registry | ExternalDatabaseVerification |

### 3.3 Forbidden outputs

Final decision outcomes (`AutoApproved`, etc.), HSRP legal verdicts, inventing text when provider missing, ExternalDatabaseVerification.

---

## 4. Flow (normative)

```text
RectifiedPlate
    │ DetectionId
    ▼
M06 Stage 3 OCR
    │
    ├─ Raw hypotheses
    ├─ Confidence
    ├─ Character evidence (if provider supports)
    └─ Provider provenance
    ▼
M06 Stage 4 Grammar
    │
    ├─ VALID
    ├─ KNOWN_INVALID
    └─ UNKNOWN_TO_REGISTRY
    ▼
GrammarResult
    │
    └── SAME DetectionId
```

---

## 5. M01 type extensions (normative — build later)

```rust
pub struct DetectionId(pub String); // from M05 / M01

pub struct ProviderRef {
    pub name: String,
    pub model: Option<String>,
}

/// Which M04 representation was OCR'd.
pub enum OcrSourceArtifact {
    ApiPrepared,
    Enhanced,
    OriginalCrop,
    Other(String),
}

pub struct CharEvidence {
    pub index: u32,
    pub glyph: String,
    pub confidence: f32,
    pub alternatives: Vec<(String, f32)>,
}

/// OCR hypothesis is NEVER a bare String.
pub struct OcrHypothesis {
    pub detection_id: DetectionId,
    pub source_artifact: OcrSourceArtifact,
    pub provider: ProviderRef,
    pub raw_text: String,
    pub normalized_text: String,
    pub confidence: f32,
    /// Present only when provider supplies it; do not fabricate.
    pub char_evidence: Option<Vec<CharEvidence>>,
}

pub enum GrammarStatus {
    Valid,
    KnownInvalid,
    UnknownToRegistry,
}

pub struct GrammarResult {
    pub detection_id: DetectionId,
    pub status: GrammarStatus,
    /// Candidate text after evidence-supported normalization (same plate identity).
    pub candidate_text: String,
    pub corrections_applied: Vec<String>, // auditable, evidence-backed only
    pub notes: Option<String>,
}

pub struct OcrGrammarBundle {
    pub detection_id: DetectionId,
    pub hypotheses: Vec<OcrHypothesis>,
    pub grammar: GrammarResult,
}
```

---

## 6. Stage 3 — OCR (locked)

1. Input: one RectifiedPlate per `DetectionId`; prefer `api_prepared` crop (config `prefer_image`).
2. Call M03 `run_ocr`; map to `OcrHypothesis[]` (cap via `max_hypotheses_per_plate`).
3. Filter by `min_hypothesis_score` without inventing replacements.
4. Soft image quality (M04 Degraded) does not alone block OCR; Unusable already halted earlier.
5. Provider transport errors follow M03 / M01 error taxonomy; never invent text on `NotImplemented`.

---

## 7. Stage 4 — Grammar (locked)

1. Always run when `grammar.enabled = true`, regardless of OCR confidence.
2. Use OCR alternatives / character evidence to justify corrections (e.g. `O↔0`).
3. Emit `GrammarStatus` as above; keep `detection_id` unchanged.
4. Do **not** mint a new plate or DetectionId.
5. `fail_open = false` (default): invalid/unknown statuses remain visible to M08.

---

## 8. Evidence

| Kind | Code | Notes |
|------|------|-------|
| VisualObservation | `ocr.s3` | Hypotheses + provenance |
| VisualObservation | `grammar.s4` | Status + corrections applied |

Never `ExternalDatabaseVerification` for grammar registry-shape checks that are local rules only. `UNKNOWN_TO_REGISTRY` means local catalog gap / unrecognized pattern — not a live VAHAN lookup.

---

## 9. Observability

- Per-stage latency (S3, S4)
- Hypotheses per DetectionId
- Grammar status counters (`VALID` / `KNOWN_INVALID` / `UNKNOWN_TO_REGISTRY`)
- Correction counts (evidence-backed only)

---

## 10. Acceptance criteria

- [ ] Every hypothesis and GrammarResult carries `DetectionId`.
- [ ] Hypothesis schema includes source artifact, provenance, raw/normalized text, confidence; char evidence optional and non-fabricated.
- [ ] Grammar never skipped solely due to high OCR confidence.
- [ ] Corrections require OCR evidence support.
- [ ] NoPlateDetected skips OCR without invented text.
- [ ] Fixture backend produces schema-valid bundles without network.
- [ ] Docs live under `Prime/Module 6/` only.

---

## 11. Build mapping (later)

| Crate | Role |
|-------|------|
| `talos-ocr` | Stage 3 + Stage 4 (grammar submodule or `talos-grammar`) |

No Rust in this docs step.

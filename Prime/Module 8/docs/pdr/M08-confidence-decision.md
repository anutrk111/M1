# PDR M08 — Confidence & Decision

| Field | Value |
|-------|-------|
| **Module ID** | M08 |
| **Module name** | Confidence & Decision |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) — APPROVED / DESIGN LOCKED |
| **Owns** | M01 **Stage 6** (observation dedup) and **Stage 7** (fusion + outcome) |
| **Depends on** | M01 outcomes/thresholds; M03 `vlm_assist`; M04 NoPlateDetected; M05 DetectionId; M06 OCR/grammar; M07 HSRP observations |
| **Layout** | [`Prime/Module 8/`](../../) |
| **Related** | [Architecture](../architecture/overview.md) · [M06](../../../Module%206/docs/pdr/M06-ocr-plate-intelligence.md) · [M07](../../../Module%207/docs/pdr/M07-hsrp-analysis.md) |

---

## 1. Purpose

M08 fuses plate-scoped evidence into a terminal decision while preserving lineage and full candidate provenance.

**Global invariant (mandatory):**

> No OCR hypothesis, grammar result, HSRP observation, VLM evidence, or final plate decision may lose its originating `DetectionId`.

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **Stage 6** | Observation dedup / clusters across imports — **≠** M05 in-frame suppression |
| **Same vehicle** | Early versions: **do not claim**; cluster only |
| **Primary plate** | Eligibility / hard constraints **first**, then rank among eligible |
| **Siblings** | Never discarded; full set in provenance |
| **Thresholds** | Config-only; `0.90`/`0.80` = starting defaults, not validated production accuracy |
| **VLM** | Selective, additive VisualObservation; cannot bypass thresholds or erase hard evidence |
| **Outcomes** | `AutoApproved` / `SecondaryVerification` / `ReviewRequired` |

---

## 3. Authority map

```text
                    DetectionId
                         │
                RectifiedPlate
                   ┌─────┴─────┐
                   ▼           ▼
             M06 OCR         M07 HSRP
             Stage 3         Stage 5
                │               │
             Stage 4            │
             Grammar            │
                └──────┬────────┘
                       ▼
                 M08 Stage 6 Observation Dedup
                       ▼
                 M08 Stage 7 Evidence Fusion
                       │
              ambiguous only → M03 VLM (VisualObservation additive)
                       ▼
            AutoApproved | Secondary | ReviewRequired
```

---

## 4. Scope

### 4.1 In scope

- Stage 6 observation clustering
- Stage 7 eligibility, ranking, fusion, outcomes
- Optional VLM assist on configured ambiguous bands
- Decision provenance including all candidates

### 4.2 Out of scope

| Owner | Capability |
|-------|------------|
| M05 | In-frame IoU detection suppression |
| M06/M07 | Producing OCR/HSRP evidence |
| M09 | Human VERIFY/CORRECT/REJECT UI |
| M10 | Durable store implementation details (M08 may consume injected batch store) |

### 4.3 Forbidden

- Unconditional max-score primary without eligibility
- Discarding sibling candidates from provenance
- Hardcoded thresholds that ignore config
- VLM overriding grammar / hard-fail evidence
- Claiming “same vehicle” in early Stage 6
- ExternalDatabaseVerification from visual path alone

---

## 5. Stage 6 — Observation deduplication (locked)

```text
M05: same frame + overlapping boxes → Detection Suppression
M08 Stage 6: different imported frames/files → Observation Deduplication → duplicate relationship / cluster
```

Signals (non-exhaustive): image hash, crop similarity, plate hypothesis, available metadata.

Output: cluster / duplicate relationship records. **Not** an unsupported same-vehicle identity claim (`claim_same_vehicle = false`).

---

## 6. Stage 7 — Fusion (locked)

### 6.1 Eligibility then rank

1. Build candidates per `DetectionId` from M06 + M07 (+ cluster context).
2. Apply hard constraints (e.g. `hard_fail_on_grammar`, `hard_fail_on_no_plate`).
3. Rank **eligible** candidates (default: highest fused score among eligible).
4. Select primary among eligible; attach **all** siblings in provenance.
5. Map fused score to outcomes using config thresholds.

### 6.2 Type sketch (build later)

```rust
pub struct PlateCandidate {
    pub detection_id: DetectionId,
    pub grammar_status: GrammarStatus,
    pub fused_score: f32,
    pub eligible: bool,
    pub ineligibility_reasons: Vec<String>,
    pub ocr_ref: Option<String>,
    pub hsrp_ref: Option<String>,
}

pub struct ObservationCluster {
    pub cluster_id: String,
    pub member_detection_ids: Vec<DetectionId>,
    pub signals: Vec<String>,
    // claim_same_vehicle must remain false in early versions
}

pub struct DecisionProvenance {
    pub candidates: Vec<PlateCandidate>, // FULL set — never drop siblings
    pub primary_detection_id: Option<DetectionId>,
    pub eligibility_notes: Vec<String>,
    pub clusters: Vec<ObservationCluster>,
    pub vlm_evidence_ids: Vec<String>,
}

pub enum DecisionOutcome {
    AutoApproved,
    SecondaryVerification,
    ReviewRequired,
}

pub struct FusedDecision {
    pub outcome: DecisionOutcome,
    pub score: f32,
    pub provenance: DecisionProvenance,
}
```

### 6.3 VLM

- Trigger from config (`secondary_band` default).
- Additive VisualObservation only.
- `can_bypass_thresholds = false`, `can_erase_hard_evidence = false`.

### 6.4 NoPlateDetected

Typed M04 signal → hard-fail config path (ADR-0018); not an M06/M07 crash.

---

## 7. Evidence

| Kind | Role |
|------|------|
| VisualObservation | OCR, HSRP, VLM, fusion notes |
| ExternalDatabaseVerification | **Not** produced by M08 visual fusion |

---

## 8. Observability

- Outcome counters
- Eligible vs ineligible candidate counts
- Cluster sizes
- VLM invocation rate
- Per-DetectionId evidence counts

---

## 9. Acceptance criteria

- [ ] DetectionId retained on all decision-facing plate artifacts.
- [ ] Eligibility applied before primary ranking.
- [ ] Full candidate set in provenance.
- [ ] Stage 6 documented as observation clusters, distinct from M05.
- [ ] Thresholds config-driven; defaults labeled starting-only.
- [ ] VLM cannot bypass thresholds or erase hard evidence.
- [ ] Docs only under `Prime/Module 8/`.

---

## 10. Build mapping (later)

| Crate | Role |
|-------|------|
| `talos-decision` | Stage 6 helpers + Stage 7 fusion |

No Rust in this docs step.

# PDR M09 — Review Management

| Field | Value |
|-------|-------|
| **Module ID** | M09 |
| **Module name** | Review Management |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) — APPROVED / DESIGN LOCKED |
| **Owns** | Human review workflow over queued decisions; ReviewEvent / ReviewedResult production |
| **Depends on** | M01 `ReviewAction` / decision types; M08 MachineDecision; M10 ReviewEventStore port; M12 auth (later) |
| **Layout** | [`Prime/Module 9/`](../../) |
| **Related** | [Architecture](../architecture/overview.md) · [M08](../../../Module%208/docs/pdr/M08-confidence-decision.md) · [M10](../../../Module%2010/docs/pdr/M10-storage-and-data.md) |

---

## 1. Purpose

M09 lets review officers act on decisions that need human attention **without overwriting** the machine decision. Auditability is the primary design goal: AI output and officer changes remain separately recoverable forever (subject to retention policy owned elsewhere).

```text
MachineDecision
      │ immutable
      ▼
ReviewEvent
      │ append-only
      ▼
ReviewedResult
      │ effective human-reviewed state
      ▼
M10 persistence → M11 export
```

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **Actions** | Only M01 `Verify` \| `Correct` \| `Reject` \| `Unreadable` |
| **MachineDecision** | Immutable — never overwritten by review |
| **ReviewEvent** | Append-only; carries officer/action/audit provenance |
| **ReviewedResult** | Effective human-reviewed state; separate entity |
| **Workflow states** | `QUEUED` / `CLAIMED` / `IN_REVIEW` / `COMPLETED` — **not** `FrameTerminalStatus` |
| **Persistence** | Via M10 ports only (`ReviewEventStore`, etc.) |
| **Auth** | Role requirements for M12; M09 does not implement IdP |

---

## 3. Scope

### 3.1 In scope

- Queueing rules for review items
- Claim / lease / version concurrency
- Acting with M01 review actions
- Correct field contract + DetectionId preservation
- Separation of VisualObservation content vs ReviewEvent provenance
- Documented REST shapes for later `review-api` routes

### 3.2 Out of scope

| Owner | Capability |
|-------|------------|
| M08 | Producing MachineDecision |
| M10 | Durable persistence, object store, reconciliation |
| M11 | Export formats |
| M12 | Login, RBAC enforcement, MFA |
| UI | Review officer frontend |

### 3.3 Forbidden

- Overwriting MachineDecision
- Inventing new FrameTerminalStatus values
- Changing DetectionId / minting detections / deleting sibling candidates
- Mixing officer identity into evidence payload
- Calling MinIO/S3/Postgres drivers directly from M09

---

## 4. Queue rules

| Outcome | Queued by default |
|---------|-------------------|
| `ReviewRequired` | Yes (`include_review_required = true`) |
| `SecondaryVerification` | Only if `include_secondary = true` |
| `AutoApproved` | Not queued |

---

## 5. Workflow (normative)

```text
QUEUED
   ↓
CLAIMED
   ↓
IN_REVIEW
   ├── Verify
   ├── Correct
   ├── Reject
   └── Unreadable
          ↓
       COMPLETED
```

- Claim acquires lease (`lease.ttl_seconds`) and version.
- Complete requires expected version (ADR-0029).
- Expired lease → reclaimable (`QUEUED`), no new FrameTerminalStatus.

---

## 6. Type sketch (build later)

```rust
pub enum ReviewWorkflowState {
    Queued,
    Claimed,
    InReview,
    Completed,
}

pub struct ReviewEvent {
    pub event_id: String,
    pub batch_id: String,
    pub frame_id: String,
    pub action: ReviewAction, // M01
    pub officer_id: String,
    pub timestamp: String, // UTC
    pub detection_id: Option<String>,
    pub previous_value: Option<String>,
    pub corrected_value: Option<String>,
    pub reason: Option<String>,
    pub note: Option<String>,
    pub expected_version: u64,
    // provenance: officer/action/audit — NOT inside Evidence payload
}

pub struct ReviewedResult {
    pub batch_id: String,
    pub frame_id: String,
    pub effective_plate_text: Option<String>,
    pub last_action: ReviewAction,
    pub derived_from_event_id: String,
    // MachineDecision id/ref remains linked, never replaced
    pub machine_decision_ref: String,
}
```

---

## 7. Correct contract (locked)

Required fields: `previous_value`, `corrected_value`, `DetectionId`, officer identity, timestamp, reason/note.

Correction **content** → optional `VisualObservation`.  
Officer/action/audit → **ReviewEvent** only.

---

## 8. Boundary with M10

```text
M09 = "What review operation happened?" → ReviewEvent
M10 = "Persist it immutably"
```

M10 does not validate correction legality or authorize the officer.

---

## 9. API surface (document now; implement later)

| Operation | Intent |
|-----------|--------|
| `GET /review/items` | List queue (filters: state, outcome, batch) |
| `POST /review/items/{id}/claim` | Claim + lease + version |
| `GET /review/items/{id}` | MachineDecision + provenance + siblings (read-only) |
| `POST /review/items/{id}/act` | Verify/Correct/Reject/Unreadable with expected_version |

Auth headers / RBAC: M12.

---

## 10. Observability

- Queue depth by workflow state
- Lease expiry / conflict counts
- Action counts by ReviewAction
- Time-in-state histograms (Timescale via M10 later)

---

## 11. Acceptance criteria

- [ ] MachineDecision never overwritten in contract docs/types.
- [ ] ReviewEvent append-only; ReviewedResult separate.
- [ ] Workflow states ≠ FrameTerminalStatus.
- [ ] Correct carries full field set; DetectionId immutable.
- [ ] Evidence payload excludes officer identity.
- [ ] Persistence only via M10 ports.
- [ ] Docs only under `Prime/Module 9/`.

---

## 12. Build mapping (later)

| Piece | Role |
|-------|------|
| `talos-review` | Domain logic |
| `review-api` routes | HTTP (extends M01 health spine) |

No Rust in this docs step.

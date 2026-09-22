# PDR M10 — Storage & Data

| Field | Value |
|-------|-------|
| **Module ID** | M10 |
| **Module name** | Storage & Data |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) — APPROVED / DESIGN LOCKED |
| **Owns** | Durable persistence, integrity, indexing, retrieval via ports |
| **Depends on** | M01 types/enums (does not redefine EvidenceKind / outcomes) |
| **Layout** | [`Prime/Module 10/`](../../) |
| **Related** | [Architecture](../architecture/overview.md) · [M09](../../../Module%209/docs/pdr/M09-review-management.md) · [M08](../../../Module%208/docs/pdr/M08-confidence-decision.md) |

---

## 1. Purpose

M10 persists Talos lineage immutably and retrieveably. It does **not** decide whether a review correction is valid or whether an officer is authorized.

**Primary invariant:**

> Database row and object-store artifact are joined by immutable lineage; the **path is not evidence identity — the hash is.**

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **SoR** | PostgreSQL for relational metadata |
| **Metrics** | TimescaleDB (or Postgres until enabled) |
| **Blobs** | MinIO/S3-compatible; fixture FS for CI |
| **Identity** | `sha256:<digest>`; logical key for humans |
| **Originals** | Write-once |
| **MachineDecision** | Immutable store |
| **ReviewEvent** | Append-only store |
| **Success** | No false persistence success (ArtifactState) |
| **Retention** | `automatic_delete = false` until formal policy |
| **Ports** | Upstream never uses driver internals |

---

## 3. Lineage graph (normative)

```text
Batch
  │
  └── Frame
       │
       ├── OriginalArtifact
       │      └── original SHA-256
       │
       ├── Detection
       │      └── DetectionId
       │
       ├── DerivedArtifact
       │      ├── PlateCrop
       │      ├── EnhancedCrop
       │      └── ApiPreparedImage
       │
       ├── OCR Evidence
       ├── HSRP Evidence
       ├── MachineDecision
       │
       ├── ReviewEvent 1
       ├── ReviewEvent 2...
       │
       └── EffectiveReviewedResult
```

---

## 4. Hybrid addressing

```text
Logical key:  {batch_id}/{frame_id}/{detection_id}/plate_crop
Integrity:    sha256:<digest>
```

---

## 5. ArtifactState (locked)

```text
PENDING → OBJECT_WRITTEN → METADATA_COMMITTED → AVAILABLE
failure → RECONCILIATION_REQUIRED
```

Incomplete writes (either order of Postgres vs object store failure) must not be treated as durable success. Export/UI promote `AVAILABLE` artifacts only (or documented equivalent).

---

## 6. Scope

### 6.1 In scope

- Port definitions and adapter responsibilities
- Metadata + blob persistence model
- ArtifactState / reconciliation contract
- Fixture store for CI
- Retention config shape (deletion off by default)

### 6.2 Out of scope

| Owner | Capability |
|-------|------------|
| M09 | Review validity, claim/lease UX |
| M12 | AuthN/Z |
| M11 | Export file formats |
| Ops | Actual cloud provisioning in this docs step |

### 6.3 Forbidden

- Redefining EvidenceKind or decision enums
- Secrets committed to the repository
- Automatic deletion while `automatic_delete = false`
- Claiming review authorization decisions inside M10
- Reporting Succeeded/AVAILABLE on incomplete writes

---

## 7. Ports (normative — build later)

```rust
#[async_trait]
pub trait FrameStore: Send + Sync { /* batch/frame terminal status */ }

#[async_trait]
pub trait BlobStore: Send + Sync {
    // put/get by logical key + returns sha256 identity
}

#[async_trait]
pub trait DecisionStore: Send + Sync {
    // immutable MachineDecision write; read by frame
}

#[async_trait]
pub trait ReviewEventStore: Send + Sync {
    // append ReviewEvent; read history; materialize ReviewedResult
}

#[async_trait]
pub trait DedupClusterStore: Send + Sync {
    // M08 Stage 6 observation clusters
}
```

Consumers:

```text
M09 → ReviewEventStore → M10
M08 → DecisionStore    → M10
M04 → BlobStore        → M10
M02 → FrameStore / BlobStore → M10
```

---

## 8. Retention

```toml
[retention]
automatic_delete = false
evidence_days = 90   # example / inactive until policy enabled
```

Originals and audit material must not be deleted by an arbitrary default. Enabling deletion requires formal department/legal policy + config flip.

---

## 9. Fixture path

`backend = "fixture"`: local filesystem under `fixture.root` + SQLite or in-memory metadata for CI. Must still honor ArtifactState semantics — never fake durable success without completing the fixture write protocol.

---

## 10. Observability

- ArtifactState counts
- Reconciliation backlog
- Blob put/get latency
- Decision / ReviewEvent append rates

---

## 11. Acceptance criteria

- [ ] Hash documented as integrity identity; logical key as navigation aid.
- [ ] MachineDecision immutable; ReviewEvent append-only in storage contract.
- [ ] ArtifactState covers both failure orders; no false success.
- [ ] `automatic_delete = false` default.
- [ ] Ports listed; drivers stay inside M10.
- [ ] Docs only under `Prime/Module 10/`.

---

## 12. Build mapping (later)

| Piece | Role |
|-------|------|
| `talos-storage` | Ports + adapters |
| Migrations | Postgres schema |
| Object client | MinIO/S3 |

No Rust / migrations / MinIO wiring in this docs step.

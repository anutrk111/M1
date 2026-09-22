# PDR M11 — Export & Reporting

| Field | Value |
|-------|-------|
| **Module ID** | M11 |
| **Module name** | Export & Reporting |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) — APPROVED / DESIGN LOCKED |
| **Owns** | Export presentation (CSV/XLSX/JSONL), summaries, ExportManifest |
| **Depends on** | M01 DecisionJson shapes; M08 MachineDecision; M09 ReviewedResult; M10 read ports; M12 permissions |
| **Layout** | [`Prime/Module 11/`](../../) |
| **Related** | [Architecture](../architecture/overview.md) · [M09](../../../Module%209/docs/pdr/M09-review-management.md) · [M10](../../../Module%2010/docs/pdr/M10-storage-and-data.md) · [M12](../../../Module%2012/docs/pdr/M12-auth-admin-config.md) |

---

## 1. Purpose

M11 turns durable M10 records into downloadable reports. It is a **presentation layer, not a decision engine**.

**Invariant:** no silent correct / normalize / replace to make values “look better.” Stored source values are never mutated.

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **Views** | `machine` \| `effective` \| `both` — explicit |
| **effective_source** | Required on effective rows: `machine` \| `human_review` |
| **Reads** | M10 ports only |
| **AuthZ** | M12 permissions (`export.read`, `export.audit`) |
| **Manifest** | Every file + ExportManifest (reproducible) |
| **Spreadsheet safety** | Serialize-safe; do not mutate storage |
| **Availability** | Default `on_reconciliation_required = flag` |

---

## 3. Views

```text
M10 Stores
   │
   ▼
M11 Export Query
   ├── machine   → immutable MachineDecision
   ├── effective → ReviewedResult if available, else MachineDecision
   └── both      → MachineDecision + ReviewedResult
```

Fallback on `effective` is allowed only with `effective_source` metadata.

---

## 4. ExportManifest (minimum)

```rust
pub struct ExportManifest {
    pub export_id: String,
    pub generated_at: String, // UTC
    pub view: String,         // machine|effective|both
    pub filters: String,      // documented JSON/TOML blob
    pub schema_version: String,
    pub row_count: u64,
    pub output_sha256: String,
}
```

Stable `order_by` for same snapshot/filter. Deterministic chunking.

---

## 5. Availability policy

```toml
[availability]
on_reconciliation_required = "flag" # flag | exclude | fail
```

Default **flag** so incomplete records are not invisible.

---

## 6. Scope

### 6.1 In scope

- Decision / review-event / summary exports
- Manifest generation and integrity hash
- Spreadsheet-safe CSV/XLSX serialization
- Documented API shapes for later implementation

### 6.2 Out of scope

| Owner | Capability |
|-------|------------|
| M08/M09 | Producing decisions / review meaning |
| M10 | Persistence / ArtifactState |
| M12 | Who may export |
| Build step | Actual calamine/xlsx crates |

### 6.3 Forbidden

- Silent overwrite of machine fields
- Inventing OCR/HSRP/ExternalDatabaseVerification
- Mutating stored values for “safety”
- Bypassing M10 ports
- Default silent exclude of `RECONCILIATION_REQUIRED`

---

## 7. Lineage columns

Always include when applicable: `batch_id`, `frame_id`, `DetectionId`, outcome, grammar/HSRP summary fields, `effective_source` (effective/both views).

---

## 8. Global authority chain

> M12 decides who may export; M11 decides what export means; M10 keeps the durable record.

---

## 9. Acceptance criteria

- [ ] Views and `effective_source` documented and required.
- [ ] ExportManifest fields locked.
- [ ] Spreadsheet safety without storage mutation.
- [ ] Availability default `flag`.
- [ ] Docs only under `Prime/Module 11/`.

---

## 10. Build mapping (later)

`talos-export` crate + API routes. No Rust in this docs step.

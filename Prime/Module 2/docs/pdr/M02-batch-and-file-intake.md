# PDR M02 — Batch & File Intake

| Field | Value |
|-------|-------|
| **Module ID** | M02 |
| **Module name** | Batch & File Intake |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) |
| **Depends on** | M01 Foundation & Core (`talos-types`, `talos-core` utilities / errors) |
| **Layout** | [`Prime/Module 2/`](../../) (sibling to Module 1) |
| **Related docs** | [Architecture overview](../../../Module%201/docs/architecture/overview.md) · [M01 PDR](../../../Module%201/docs/pdr/M01-foundation-and-core.md) |

---

## 1. Purpose

M02 turns raw batch inputs (image folders, ZIP archives, HTTP uploads, optional CSV/XLSX sidecars) into validated, content-addressed **`IntakeEnvelope`** records (JSON schema v2.0) ready for the M01 Stage 0–7 pipeline.

M02 **produces** envelopes defined by M01. It does **not** redefine schemas, run inference, call AI providers, or persist long-term storage.

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **Primary mode** | API-first, **batch / file intake** |
| **Not live streams** | No live camera access, SFTP camera pull, or MQTT frame feeds |
| **Schema ownership** | `IntakeEnvelope`, IDs, `FrameMetadata`, `TalosError` live in **M01** |
| **Content addressing** | Every accepted image has SHA-256; `bytes_ref` points at staged bytes |
| **No silent drops** | An accepted frame must appear on the batch manifest with a terminal intake status |
| **Durable queue** | Handoff trait only; RabbitMQ/NATS adapters are later — `NotImplemented` until wired |

---

## 3. Scope

### 3.1 In scope

| Capability | Description |
|------------|-------------|
| Folder intake | Recurse a batch root for allowed image extensions |
| ZIP intake | Safe extract to staging; reject zip-slip and empty archives |
| Upload intake | Multipart HTTP (or equivalent) with same image rules |
| Metadata sidecar | Optional CSV/XLSX → `FrameMetadata` join |
| Hashing & IDs | SHA-256 via M01 helpers; allocate `BatchId` / `FrameId` |
| Envelope emission | One `IntakeEnvelope` per accepted image |
| Batch manifest | Counts, per-frame intake status, errors / warnings |
| Limits & validation | Config-driven size/count/content-type guards |
| Frame handoff | `FrameSink` trait to push envelopes downstream |

### 3.2 Out of scope

| Owner | Capability |
|-------|------------|
| M01 | Pipeline Stages 0–7, decision thresholds, shared types |
| M03 | Vision / VLM API calls |
| M04–M08 | Detection, OCR, HSRP, fusion logic |
| M10 | PostgreSQL / TimescaleDB / MinIO durable object storage |
| M12 | Authentication / authorization of upload callers |
| — | Inventing OCR text, detections, or decision outcomes |

### 3.3 Non-goals

- Redefining `IntakeEnvelope` or parallel “intake v3” schemas
- Silently skipping corrupt files without manifest entries
- Live ANPR camera adapters
- Pretend durable-queue success when no broker is configured

---

## 4. Goals and success criteria

### 4.1 Goals

1. Every accepted image → exactly one `IntakeEnvelope` with `schema_version = "2.0"`.
2. `image.sha256` matches the hashed bytes; `bytes_ref` is `file://…` or `object://…`.
3. Metadata join is deterministic (rules in §7).
4. Unsupported / unsafe inputs fail loudly via M01 `TalosError` classes.
5. Module 2 code depends on Module 1 crates via path; never forks types.

### 4.2 Acceptance criteria (future implementation)

- [ ] Fixture folder with N JPG/PNG files yields N envelopes (or N−D after in-batch SHA dedupe) and a complete `BatchManifest`.
- [ ] Zip-slip paths are rejected; staging root is never escaped.
- [ ] Malformed metadata rows fail **that row** and continue the batch; warning count recorded.
- [ ] Empty ZIP / zero images after filter → `Validation` or empty-batch policy as configured (default: fail batch).
- [ ] `FrameSink` in-memory implementation works in tests; durable queue methods return `NotImplemented` until provided.

---

## 5. Crate and app mapping (implementation binding)

Documented now; **not built in the PDR-only step**.

| Concern | Future location |
|---------|-----------------|
| Intake library | `Prime/Module 2/crates/talos-intake` |
| Intake config | `Prime/Module 2/configs/intake.toml` + `TALOS__INTAKE__*` |
| HTTP batch endpoints | Extend Module 1 `talos-api` **or** Module 2 `talos-intake-api` |
| CLI | `talos-worker intake …` and/or `talos-intake-cli` |
| Fixtures | `Prime/Module 2/tests/fixtures/` |

```text
talos-intake ──depends──► talos-core ──► talos-config
                │              └──► talos-types
                └──depends──────────────┘
```

---

## 6. Input modes (normative)

| Mode | `SourceKind` | Behavior |
|------|--------------|----------|
| Folder path | `folder` | Recurse batch root for `.jpg` / `.jpeg` / `.png` (case-insensitive); ignore other files |
| ZIP path or upload | `zip` | Extract to staging directory under config root; apply same image discovery |
| Multipart HTTP | `upload` | Accept image parts (+ optional metadata part); stage then discover |

### 6.1 Default limits (config-overridable)

| Key | Default | Meaning |
|-----|---------|---------|
| `max_images_per_batch` | `10000` | Hard cap on accepted frames per batch |
| `max_zip_bytes` | `512 MiB` | Reject larger ZIP before extract |
| `max_image_bytes` | `25 MiB` | Reject oversized single image |
| `allowed_content_types` | `image/jpeg`, `image/png` | Content-type / magic sniff must agree |
| `fail_on_empty_batch` | `true` | No accepted images → batch failure |
| `staging_root` | `./data/staging` | Extract / upload staging base |

### 6.2 ZIP safety (mandatory)

1. Reject entries whose resolved path escapes `staging_root/batch_id/` (zip-slip).
2. Reject symbolic links inside ZIP (or refuse to follow them).
3. Enforce `max_zip_bytes` on compressed size **and** a configured uncompressed budget if set.
4. Empty ZIP (no files) → validation failure when `fail_on_empty_batch` is true.

---

## 7. Metadata (CSV / XLSX)

### 7.1 Sidecar role

Optional. Absence of sidecar is valid: envelopes emit with empty / null `FrameMetadata`.

### 7.2 Column mapping

| Sidecar column (aliases allowed) | `FrameMetadata` field |
|----------------------------------|----------------------|
| `camera_id` / `camera` | `camera_id` |
| `captured_at` / `timestamp` / `ts` | `captured_at` (RFC 3339 UTC preferred) |
| `location` / `loc` | `location` |
| `path` / `file` / `relative_path` | **Join key** (not stored as metadata field) |

`CameraId` remains optional metadata only — **not** a live feed handle (M01 rule).

### 7.3 Join rules (deterministic)

1. **Primary key:** relative path within the batch root, using `/` separators, matched case-sensitively to the sidecar `path` / `relative_path` value after normalization (strip leading `./`).
2. **Fallback:** if no relative-path match, join on **basename** (file name only). If multiple images share a basename, basename join applies only when exactly one candidate remains; otherwise leave metadata null and record a warning.
3. **Unmatched images:** allowed — envelope with null metadata fields.
4. **Malformed rows:** fail **that row** (record in manifest warnings/errors); **continue** the batch. Do not abort the entire batch for a single bad CSV line unless `fail_batch_on_metadata_errors = true` (default **false**).

### 7.4 Example sidecar (CSV)

```csv
relative_path,camera_id,captured_at,location
cam01/img_0001.jpg,CAM-01,2026-09-22T05:00:00Z,Raipur
cam01/img_0002.jpg,CAM-01,2026-09-22T05:00:01Z,Raipur
```

---

## 8. Output contracts

### 8.1 IntakeEnvelope (M01 — normative shape)

M02 **must** emit M01’s envelope without field renames:

```json
{
  "schema_version": "2.0",
  "batch_id": "01J…",
  "frame_id": "01J…",
  "source": { "kind": "folder", "path_or_key": "cam01/img_0001.jpg" },
  "image": {
    "content_type": "image/jpeg",
    "sha256": "…",
    "bytes_ref": "file:///…/staging/…/img_0001.jpg"
  },
  "metadata": {
    "camera_id": "CAM-01",
    "captured_at": "2026-09-22T05:00:00Z",
    "location": "Raipur"
  }
}
```

`source.kind` ∈ { `folder`, `zip`, `upload` } per M01 `SourceKind`.

### 8.2 BatchManifest (M02-defined)

```json
{
  "schema_version": "2.0",
  "batch_id": "01J…",
  "source_summary": { "kind": "zip", "path_or_key": "uploads/day1.zip" },
  "counts": {
    "discovered": 100,
    "accepted": 97,
    "rejected": 2,
    "skipped_duplicate": 1,
    "metadata_warnings": 3
  },
  "frames": [
    {
      "frame_id": "01J…",
      "relative_path": "cam01/img_0001.jpg",
      "sha256": "…",
      "intake_status": "Accepted",
      "envelope": { }
    }
  ],
  "errors": [],
  "warnings": []
}
```

#### Intake frame status

| Status | Meaning |
|--------|---------|
| `Accepted` | Envelope emitted / handed to `FrameSink` |
| `RejectedValidation` | Bad type, size, path, etc. |
| `RejectedPermanent` | Corrupt bytes / unreadable image |
| `SkippedDuplicate` | Same SHA-256 already accepted in this batch |
| `FailedSink` | Envelope built but handoff failed (must be visible — no silent drop) |

### 8.3 FrameSink (handoff)

```rust
#[async_trait]
pub trait FrameSink: Send + Sync {
    async fn submit(&self, envelope: IntakeEnvelope) -> Result<(), TalosError>;
}
```

| Implementation | Status in early M02 |
|----------------|---------------------|
| In-memory / channel | Required for tests and local CLI |
| Durable queue (NATS/RabbitMQ) | `NotImplemented` until queue module lands |
| Direct pipeline run | Optional adapter; must still record manifest status |

---

## 9. Processing flow

```text
1. Create BatchId; create staging dir under staging_root/batch_id/
2. Ingest source (copy folder listing / extract ZIP / save uploads)
3. Discover candidate image paths
4. For each candidate:
   a. Validate path safety, size, content-type / magic
   b. Hash SHA-256; if duplicate in batch → SkippedDuplicate
   c. Allocate FrameId; build IntakeEnvelope (+ metadata join)
   d. FrameSink::submit; on success → Accepted; on sink error → FailedSink
5. Write BatchManifest; return to caller
```

Tracing spans (M01 conventions): `batch.run` (intake), fields `batch_id`, `trace_id`; per frame `frame_id`, `relative_path`.

---

## 10. Errors and safety

### 10.1 Mapping to M01 `TalosError`

| Situation | Class |
|-----------|-------|
| Bad CSV header, oversize image, disallowed type, zip-slip | `Validation` |
| Unreadable/corrupt image bytes | `Permanent` |
| Staging I/O / disk full (retryable) | `Transient` |
| Durable sink not configured | `NotImplemented` |
| Invariant broken in intake code | `Internal` |

### 10.2 Idempotency

Within a single batch, identical `sha256` → at most one `Accepted` envelope; subsequent paths → `SkippedDuplicate` on the manifest.

Cross-batch dedupe is **not** M02’s job (Stage 6 / M10).

### 10.3 No silent drop

Once a frame is classified `Accepted`, it must either:

1. Successfully reach `FrameSink::submit`, or  
2. Be recorded as `FailedSink` with error detail.

Never acknowledge HTTP/CLI success for a batch while accepted frames are missing from both sink and manifest.

---

## 11. Configuration

**Load order:** defaults → `configs/intake.toml` → `TALOS__INTAKE__*` env (aligned with M01 config style).

### 11.1 Keys

```toml
[intake]
staging_root = "./data/staging"
max_images_per_batch = 10000
max_zip_bytes = 536870912
max_image_bytes = 26214400
fail_on_empty_batch = true
fail_batch_on_metadata_errors = false
allowed_extensions = ["jpg", "jpeg", "png"]
```

Secrets: none required for local folder/ZIP. Upload auth is M12.

---

## 12. Observability

| Name | Type | Labels |
|------|------|--------|
| `talos_intake_frames_total` | counter | `status` (`accepted`, `rejected`, `skipped_duplicate`, …) |
| `talos_intake_batch_duration_seconds` | histogram | `source_kind` |
| `talos_intake_bytes_total` | counter | `direction` (`read`) |

Log fields: `trace_id`, `batch_id`, `frame_id`, `relative_path`, `schema_version`.

---

## 13. Interfaces to other modules

| Module | Relationship |
|--------|----------------|
| **M01** | Supplies types, IDs, hashing, errors, tracing/health bootstrap |
| **M03–M08** | Consume envelopes indirectly via pipeline; M02 does not call them |
| **M10** | Later may replace `file://` staging with `object://` uploads; M02 keeps `bytes_ref` opaque |
| **M12** | Authenticates HTTP intake callers |
| **talos-api / worker** | Surfaces batch submit + status using `BatchManifest` |

**Must not:** own a parallel envelope schema; write `ExternalDatabaseVerification` evidence; run Stages 0–7 inside intake.

---

## 14. Security and compliance

1. Reject path traversal in ZIP and relative upload names.
2. Do not execute or interpret non-image ZIP members beyond skip/ignore.
3. Stage files with restrictive permissions where the OS allows.
4. Plate text is not produced by M02; still avoid logging full paths that embed PII if policy requires redaction.

---

## 15. Testing requirements (future implementation)

| Layer | What |
|-------|------|
| Unit | Zip-slip rejection; SHA-256 vectors; metadata join primary/fallback; extension filter |
| Contract | Envelope serde round-trip equals M01 types; manifest status enum completeness |
| Integration | Fixture folder → N envelopes + manifest; ZIP fixture; CSV join fixture |
| Negative | Empty ZIP; oversize image; `NotImplemented` durable sink |

Tests must not require network, GPU, VAHAN, or a real message broker.

---

## 16. Documentation and ADR companions

| Document | Purpose |
|----------|---------|
| `docs/pdr/M02-batch-and-file-intake.md` | This PDR (normative) |
| `docs/adr/0004-intake-metadata-join.md` | Relative-path primary + basename fallback (at implementation) |
| `docs/adr/0005-intake-no-silent-drop.md` | Accepted ⇒ sink or `FailedSink` (at implementation) |

---

## 17. Revision history

| Version | Date | Notes |
|---------|------|-------|
| 1.0 | 2026-09-22 | Initial normative PDR for M02 Batch & File Intake |

---

## 18. Open extension points (explicitly deferred)

| Extension | Status |
|-----------|--------|
| Live ANPR / camera stream ingest | Out of scope |
| SFTP / MQTT pull adapters | Out of scope |
| Durable NATS / RabbitMQ sink | Trait reserved; `NotImplemented` until queue module |
| Automatic MinIO put on intake | M10 collaboration; optional later |
| XLSX formulas / multi-sheet complexity | Support simple first sheet only when implemented |

These must not be faked inside M02 to appear complete.

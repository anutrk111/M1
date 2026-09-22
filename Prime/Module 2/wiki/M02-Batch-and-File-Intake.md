# M02 Batch & File Intake (PDR)

| Field | Value |
|-------|-------|
| **Module ID** | M02 |
| **Module name** | Batch & File Intake |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) — wiki mirror |
| **Depends on** | M01 Foundation & Core |
| **Canonical source** | [`docs/pdr/M02-batch-and-file-intake.md`](../docs/pdr/M02-batch-and-file-intake.md) |

> Prefer the repo PDR for diffs and reviews. This page summarizes the same decisions for wiki browsing.

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

## 3. Scope (summary)

**In scope:** folder / ZIP / upload intake, optional metadata sidecar, hashing & IDs, envelope emission, batch manifest, limits & validation, `FrameSink` handoff.

**Out of scope:** M01 pipeline Stages 0–7, M03 vision APIs, M04–M08 detection/OCR/HSRP/fusion, M10 durable object storage, M12 auth, inventing OCR/detections/decisions.

---

## 4. Input modes

| Mode | `SourceKind` | Behavior |
|------|--------------|----------|
| Folder path | `folder` | Recurse for `.jpg` / `.jpeg` / `.png` |
| ZIP path or upload | `zip` | Safe extract to staging; reject zip-slip |
| Multipart HTTP | `upload` | Stage image parts (+ optional metadata); same image rules |

Default limits (config-overridable): `max_images_per_batch=10000`, `max_zip_bytes=512 MiB`, `max_image_bytes=25 MiB`, `fail_on_empty_batch=true`, `staging_root=./data/staging`.

---

## 5. Metadata join (ADR 0004)

1. **Primary:** relative path within batch root (`/` separators, case-sensitive, strip leading `./`).
2. **Fallback:** basename only when exactly one candidate remains; else null metadata + warning.
3. Unmatched images allowed. Malformed rows fail that row and continue (default).

See [ADR 0004](../docs/adr/0004-intake-metadata-join.md).

---

## 6. Outputs

- **`IntakeEnvelope`** — M01 shape, `schema_version = "2.0"`.
- **`BatchManifest`** — counts + per-frame `intake_status`: `Accepted`, `RejectedValidation`, `RejectedPermanent`, `SkippedDuplicate`, `FailedSink`.
- **`FrameSink`** — `submit(envelope)`; in-memory for tests; durable queue `NotImplemented` until wired.

---

## 7. No silent drop (ADR 0005)

Once classified `Accepted`, a frame must reach `FrameSink::submit` **or** be recorded as `FailedSink`. Never acknowledge batch success while accepted frames are missing from both sink and manifest.

See [ADR 0005](../docs/adr/0005-intake-no-silent-drop.md).

---

## 8. Configuration

Load order: defaults → `configs/intake.toml` → `TALOS__INTAKE__*`.

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

---

## 9. Implementation status

Documented crate binding (`talos-intake`) is **not built** yet. Fixtures will land under `tests/fixtures/`.

Full normative detail: [M02 PDR](../docs/pdr/M02-batch-and-file-intake.md).

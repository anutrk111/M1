# M02 Implementation Report

## Starting SHA

`804eafc6814d4638493f168da2bd16238805635b`

## Final local SHA

`394b4f11f8728300fba9fa36ef8f12222ae1690b` (feat commit; report SHA update follows in a docs commit)

## Files created

- `Prime/Module 2/crates/talos-intake/` — full crate (config, discover, image, metadata, zip_safe, sink, pipeline, types)
- `Prime/Module 2/crates/talos-intake/tests/intake_integration.rs`
- `Prime/Module 1/crates/talos-intake` — workspace path symlink for Cargo workspace inheritance
- `Prime/Module 2/docs/M02_IMPLEMENTATION_REPORT.md` (this file)

## Files modified

- `Prime/Module 1/Cargo.toml` / `Cargo.lock` — workspace member `talos-intake`
- `Prime/Module 2/configs/intake.toml` — zip bomb budget keys
- `Prime/Module 2/README.md`, `Prime/README.md` — status Implemented
- `.github/workflows/ci.yml` — job name covers M01+M02

## Architecture implemented

```text
Folder/ZIP → discover → validate/hash → in-batch SHA dedupe
  → FrameId + IntakeEnvelope (+ metadata join) → FrameSink
  → BatchManifest (reconciliation authority)
```

No OCR/AI/detection/HSRP/review/Postgres/MinIO/UI.

## Domain types added (M02-owned)

`IntakeStatus`, `BatchManifest`, `BatchCounts`, `FrameRecord`, `DuplicateReference`, `MetadataRecord`, `ImportRequest`/`ImportResult`, `IntakeConfig`, `FrameSink`.

Reused M01: `IntakeEnvelope`, `BatchId`, `FrameId`, `SourceKind`, `TalosError`, `sha256_hex`, ID helpers.

## Folder intake

Deterministic `WalkDir` (no symlink follow), case-insensitive jpg/jpeg/png, unsupported files counted as `ignored_unsupported` (not accepted). Staging copy preserves original bytes.

## ZIP behavior

Safe extract under `staging_root/batch_id/`; reject `..`, absolute paths, symlinks; enforce `max_zip_bytes`, `max_zip_entries`, `max_uncompressed_zip_bytes`.

## Metadata behavior

CSV + first-sheet XLSX; join relative path primary, unique basename fallback; ambiguous basename → null metadata + warning; malformed row → warning/continue (default).

## Duplicate behavior

In-batch SHA-256: first `Accepted`, later `SkippedDuplicate` with `DuplicateReference.original_frame_id`. Files not deleted/mutated.

## FrameSink

`InMemoryFrameSink` required; sink error → `FailedSink` on manifest (no silent drop). Durable queues not implemented.

## Manifest reconciliation

`image_candidates_discovered == accepted + rejected + skipped_duplicate + failed_sink`
and `filesystem_entries_seen == image_candidates_discovered + ignored_unsupported` (`reconcile_ok`).

## Hardening (post-MVP)

See [M02_HARDENING_REPORT.md](M02_HARDENING_REPORT.md) for:

- Prime workspace migration (Module 1 nested workspace removed; lockfile at `Prime/Cargo.lock`)
- `BatchOutcome` + `fail_batch_on_sink_errors`
- Clarified `BatchCounts` accounting fields
- `image` crate decode validation (magic → decode, no re-encode; pixel limits)
- Staging lifecycle: M02 owns staging for intake; cleanup deferred to orchestrator/M10

## Security controls

Zip-slip / absolute ZIP paths / symlink reject; size/count caps; magic + decode validation; no network; no secrets in config.

## Tests added

Integration coverage: JPG/JPEG/PNG accept, unsupported, corrupt/fake-magic/truncated, order, SHA/immutability, duplicates, CSV join, basename/ambiguous, malformed metadata, missing metadata, ZIP import, zip-slip, absolute ZIP, sink failure / `BatchOutcome`, accounting invariants, envelope serde, config load.

## Verification (executed)

| Command | Exit |
|---------|------|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 |
| `cargo test --workspace --all-features` | 0 (44 passed; see hardening report for post-MVP) |
| `cargo check --workspace --all-targets --all-features` | 0 |

*(Hardening re-verification: see M02_HARDENING_REPORT.md.)*

## Known limitations / deferred

- HTTP multipart upload API surface (same validation path when wired)
- Durable queue `FrameSink` adapters → `NotImplemented` until queue module
- Multi-sheet XLSX complexity (first sheet only)
- Continuous folder watching (explicitly non-MVP)

## PDR deviations

None material. Upload mode deferred as HTTP surface only; library rules ready.

## Ready for M03 integration?

**Yes** for envelope production: M02 emits M01 `IntakeEnvelope` v2.0 into `FrameSink`. M03 is not called by M02; downstream pipeline can consume envelopes asynchronously.

## Final state

**M02 GREEN WITH DOCUMENTED DEFERRED ITEMS** (upload HTTP + durable sink deferred; core folder/ZIP/metadata/dedupe/sink/manifest green).

---

## Wave 1 hardening addendum

Branch `impl/intake-hardening` (based on `integration/m01-m03`). No changes to `talos-types` (ADR-0042 frozen contracts) or any other module; `TalosError` and M01 IDs/envelopes reused throughout.

### DurableSink boundary (`sink.rs`)

```rust
pub struct IdempotencyKey { pub frame_id: FrameId, pub sha256: String }
pub enum SinkAck { Accepted, Duplicate }

#[async_trait]
pub trait DurableSink: Send + Sync {
    async fn offer(&self, envelope: IntakeEnvelope, key: IdempotencyKey)
        -> Result<SinkAck, TalosError>;
}
```

- **Ack**: `Ok(SinkAck::Accepted)` (newly stored) or `Ok(SinkAck::Duplicate)` (key already stored — e.g. retry after a lost ack; nothing stored twice).
- **Nack**: `Err(TalosError)` — `Transient` = retryable, `Permanent`/`Validation` = poison, `NotImplemented` = no backend. No separate `Nack` variant, so every nack carries an M01 error class.
- `InMemoryDurableSink` dedupes by `IdempotencyKey` and nacks with `Validation` if the key does not match the envelope's `frame_id`/`image.sha256`.
- `UnconfiguredDurableSink` always nacks `NotImplemented` (PDR §10.1 “durable sink not configured”).
- `DurableSinkAdapter<D>` implements `FrameSink`: `Accepted`/`Duplicate` → frame `Accepted`; nack → frame `FailedSink`, batch `PartialFailure` (or batch `Err` when `fail_batch_on_sink_errors = true`) — identical to existing `FrameSink` failure semantics. `duplicate_acks()` exposes how many duplicate acks were seen.

### Upload adapter (`upload.rs`, `pipeline.rs`)

`import_upload(files: Vec<UploadedFile>, config, sink, metadata_path)` with `UploadedFile { file_name, bytes, relative_dir: Option<String> }`.

- Folder, ZIP and upload now all go through `import_source(IntakeSource, …)` → one shared `run_batch` (validation, decode check, SHA-256, in-batch dedupe, metadata join, sink hand-off, manifest + reconciliation). `import_folder` / `import_zip` keep their signatures and behaviour.
- `SourceKind::Upload`; bytes persisted unchanged to `staging_root/<batch_id>/<relative>` and verified by re-read.
- File names containing `/`, `\`, NUL, `..`, `.`, empty, drive prefix, or unsafe `relative_dir` (`..`/absolute) → `RejectedValidation` (never written to disk). Duplicate upload paths → `RejectedValidation`. Oversize (`max_image_bytes`) → `RejectedValidation` before staging. `max_images_per_batch` enforced by the shared loop. Unsupported extensions → `ignored_unsupported`, as for folders. `filesystem_entries_seen` = number of uploaded files.

### Recovery / idempotent re-run (`recovery.rs`)

`FrameId` is minted by `talos_core::util::new_frame_id()` — a **random ULID** — so it cannot identify the same file across runs. Recovery therefore keys on **`(relative_path, sha256)`** (`RecoveryKey`):

- `RecoveryContext::from_manifest(&prior)` (or `RecoveryContext::new(batch_id, kind).with_accepted(..)/with_retry(..)`); pass it as `import_source(source, …, Some(&ctx))`.
- The re-run reuses the prior `batch_id` and staging dir. Staging never overwrites differing bytes (folder copy, upload write, and ZIP extract all compare against an existing staged file and fail rather than overwrite evidence).
- Prior `Accepted` / `SkippedAlreadyAccepted` keys → new status `IntakeStatus::SkippedAlreadyAccepted` (prior `FrameId` recorded), **not** re-emitted, counted in `skipped_already_accepted`. They still consume `max_images_per_batch` budget.
- Prior `FailedSink` keys are retried **with their original `FrameId`**, so the durable `IdempotencyKey` (`FrameId` + sha256) is stable and a lost-ack retry resolves to `SinkAck::Duplicate` instead of a second copy.
- Same content at a new path → `SkippedDuplicate` referencing the prior accepted `FrameId`.
- A re-run where everything was already accepted is not an “empty batch” (`fail_on_empty_batch` only fires when `accepted == 0 && skipped_already_accepted == 0`).
- Source kind must match the prior manifest (`Validation` otherwise). The returned manifest describes the re-run only.

### Counters / reconciliation

New `BatchCounts::skipped_already_accepted` (`#[serde(default)]`, so older manifests still deserialize).

```text
image_candidates_discovered == accepted + rejected + skipped_duplicate
                               + failed_sink + skipped_already_accepted
filesystem_entries_seen     == image_candidates_discovered + ignored_unsupported
```

### Staging lifecycle (`config.rs`, `staging.rs`)

`IntakeConfig::staging_policy: StagingPolicy` (`retain` default | `plan_only` | `plan_batch_dir`), also in `configs/intake.toml`.

`plan_cleanup(manifest, staging_root, policy) -> CleanupPlan` is **pure** (no filesystem access, never deletes):

| Policy | Plan |
|--------|------|
| `retain` | empty |
| `plan_only` | staged copies of `RejectedValidation`, `RejectedPermanent`, `SkippedDuplicate` frames (never `Accepted`, `FailedSink`, `SkippedAlreadyAccepted`) |
| `plan_batch_dir` | the whole `staging_root/<batch_id>/`; executor must first confirm accepted bytes are durably persisted elsewhere |

Only paths that stay under `staging_root/<batch_id>/` are listed (unsafe relative paths / batch ids skipped). Entries may name files that were never staged (uploads rejected pre-staging), so executors treat missing paths as no-ops. **Executing the plan belongs to M10 / the orchestrator.**

### Other hardening

- ZIP entry read/decompress/CRC failures now map to `Validation` (corrupt input) instead of `Transient`; disk write failures stay `Transient`.

### Tests added

`tests/intake_hardening.rs` (13): corrupt ZIP central directory (smashed CD headers, truncated EOCD, bad CD offset) → `Validation`, no panic, nothing emitted; recovery re-run emits 0 frames and counts `skipped_already_accepted` (plus chained recovery); recovery retries `FailedSink` with the same `FrameId`; ZIP recovery idempotent; recovery source-kind mismatch; legacy manifest without new counter deserializes; upload/folder parity (same counts, statuses, sha256s; `SourceKind::Upload`; staged bytes verified); upload path traversal rejected (nothing written outside staging); upload size/count limits; DurableSink nack (`Transient`/`Permanent`/`NotImplemented`) → `FailedSink` + `PartialFailure`, and `Err` with `fail_batch_on_sink_errors`; DurableSink accepts once with in-batch dup; DurableSink duplicate key not double-stored (lost ack + recovery); `plan_cleanup` lists expected paths and deletes nothing.

Unit (new): `sink` (4), `staging` (3), `upload` (2), `config::parses_staging_policy`. Existing tests kept (one assertion added to `loads_repo_intake_toml`).

### Verification (from `Prime/`, `CARGO_TARGET_DIR=Prime/target`)

| Command | Exit |
|---------|------|
| `cargo fmt --all -- --check` | 0 |
| `cargo check --workspace --all-targets --all-features` | 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 |
| `cargo test --workspace --all-features` | 0 (85 passed; talos-intake 54 = 20 unit + 13 hardening + 21 integration) |
| `git diff --check` | 0 |

### Deferred

- Real durable queue backend (NATS/RabbitMQ) implementing `DurableSink` → **M10**.
- HTTP multipart upload route calling `import_upload` → **M12** (`talos-api`, auth).
- Executing `CleanupPlan` deletions → **M10 / orchestrator**.
- Deterministic `FrameId` minting (would need an M01 contract change); recovery keys on `(relative_path, sha256)` instead.
- ZIP uncompressed budget still trusts declared entry sizes.

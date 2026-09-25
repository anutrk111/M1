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

Deterministic `WalkDir` (no symlink follow), case-insensitive jpg/jpeg/png, unsupported files counted as `ignored_unsupported` (not accepted). Staging copy preserves original bytes. Since the PR #6 remediation, files are copied lazily and only after the size and batch-limit checks (see addendum).

## ZIP behavior

Vet the archive under `staging_root/batch_id/`: reject `..`, absolute paths, symlinks; enforce `max_zip_bytes`, `max_zip_entries`, `max_uncompressed_zip_bytes`. Since the PR #6 remediation, image entries are extracted on demand and non-image entries are counted but never extracted (see addendum).

## Metadata behavior

CSV + first-sheet XLSX; join relative path primary, unique basename fallback; ambiguous basename → null metadata + warning; malformed row → warning/continue (default).

## Duplicate behavior

In-batch SHA-256: first `Accepted`, later `SkippedDuplicate` with `DuplicateReference.original_frame_id`. Files not deleted/mutated. Since the PR #6 remediation, only a sink-acknowledged frame (or a prior accepted frame during recovery) can be the duplicate original.

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
- ZIP uncompressed budget still trusts declared entry sizes. (Per-entry extraction is now capped while streaming; see the PR #6 addendum.)

## PR #6 remediation addendum

Fixes from the PR #6 review, verified against `integration/m01-m03` @ `78d8860`.

### Pre-staging resource limits (`pipeline.rs`, `staging.rs`, `zip_safe.rs`, `upload.rs`)

**Cause.** Folder intake copied every candidate into staging (`stage_copies`), and ZIP intake extracted every entry, before `run_batch` applied `max_image_bytes` or `max_images_per_batch`. An oversize file was copied and then rejected; a folder of N images with a limit of 2 staged all N. `stage_file` also held source and destination fully in memory to prove immutability.

**Fix.**
- Candidates are now `Pending` (`Source(path)`, `Bytes(upload)`, `ZipEntry(index)`) and staged inside the batch loop, after the `max_images_per_batch` check. Excess candidates are recorded as `RejectedValidation` (`max_images_per_batch exceeded`) without being copied, written, extracted, hashed or decoded. Candidate order is unchanged (sorted relative path), so recovery stays deterministic.
- Folder files over `max_image_bytes` are rejected from `fs::metadata` before being opened; ZIP entries from their declared size; uploads from their length. Copies are also capped while streaming, so a file that grows mid-copy cannot exceed the cap.
- Staging is bounded-memory: stream into a partial file while hashing, publish with a hard link (fails instead of overwriting), then stream-hash the destination and compare digest and length. Folder sources are re-hashed after the copy to detect a source that changed mid-copy. The in-loop re-read of the whole file is replaced by a streaming re-hash. Partial files are this run's own scratch output, never evidence.
- ZIP: `plan_zip` vets names, symlinks, entry count and declared budget up front (batch-level `Validation`, as before) and keeps the archive open; entries are extracted one at a time. Non-image entries are counted in `ignored_unsupported` and never extracted. A repeated entry name (e.g. `a.jpg` and `./a.jpg`) is a per-frame `RejectedValidation`, never an overwrite. A corrupt entry body (CRC / decompress) is now a per-frame `RejectedValidation`, where it used to fail the whole batch; central-directory corruption still fails the batch.
- Staging I/O failures (`Transient` / `Internal`) still abort the batch rather than being misfiled as rejections.
- The batch tracing span is attached with `Instrument` instead of an entered guard held across `.await`, which made the intake future non-`Send`.

### Sink-acknowledged duplicate authority (`pipeline.rs`)

**Cause.** A fresh frame's SHA was inserted into `seen_sha` before `sink.submit`. If the submit failed, a later byte-identical file became `SkippedDuplicate` of a frame that was never accepted, leaving no accepted original.

**Fix.** The SHA is inserted only on a successful sink ack (`Ok`, which includes a durable `Duplicate` ack). Recovery still seeds authority from prior `Accepted` / `SkippedAlreadyAccepted` frames and retries `FailedSink` frames with their original `FrameId`. If a later copy was accepted in the failed run, the recovery run records the earlier `FailedSink` path as `SkippedDuplicate` of that accepted frame rather than storing the content twice.

### Image limit classification (`image.rs`)

**Cause.** Width, height and allocation limits were passed to the decoder, and every decode error, including the decoder's `Limits` error, mapped to `Permanent`.

**Fix.** Header dimensions are read first and checked against `max_width` / `max_height` / `max_pixel_count` (`Validation`). Full decode remains mandatory; a decoder `Limits` error (limits come only from config) is `Validation`, any other decode failure is `Permanent`, and decoded dimensions must equal the header's. `max_alloc` now budgets 16-bit RGBA (8 bytes per pixel).

### Tests

`tests/pr6_remediation.rs` (10): oversize folder file rejected before staging (not hashed, not staged, original untouched); `max_images_per_batch = 2` stages only the admitted folder, ZIP and upload candidates; ZIP oversize entry and repeated name rejected per frame; a tampered staged copy is never overwritten on recovery; a sink-failed first copy does not suppress later same content (fails on the previous logic); all-failed duplicates retry with the original `FrameId` (fails on the previous logic); width / height / pixel limits vs truncated / fake magic map to `RejectedValidation` vs `RejectedPermanent`; the intake future is `Send`. Unit: `image::classification_matrix` (valid JPEG / PNG, corrupt, truncated, width, height, pixel count, exact limits), `decoder_limit_errors_map_to_validation`, `sixteen_bit_png_within_pixel_budget_is_accepted`.

talos-intake now has 67 tests: 23 unit, 13 hardening, 21 integration, 10 remediation.

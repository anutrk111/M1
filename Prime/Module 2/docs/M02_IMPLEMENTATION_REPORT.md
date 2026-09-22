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

`discovered == accepted + rejected + skipped_duplicate + failed_sink` enforced (`reconcile_ok`).

## Security controls

Zip-slip / absolute ZIP paths / symlink reject; size/count caps; no network; no secrets in config.

## Tests added

Integration coverage: JPG/JPEG/PNG accept, unsupported, corrupt, order, SHA/immutability, duplicates, CSV join, basename/ambiguous, malformed metadata, missing metadata, ZIP import, zip-slip, absolute ZIP, sink failure, envelope serde, config load.

## Verification (executed)

| Command | Exit |
|---------|------|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 |
| `cargo test --workspace --all-features` | 0 (36 passed) |
| `cargo check --workspace --all-targets --all-features` | 0 |

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

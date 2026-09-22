# M02 Hardening Report

## Starting SHA

`07e4323dfd1e07c7220798b8117c50d386c33df1` (synced with `origin/main`)

## Final SHA

`d5acafd06e49b5b3a15ce55c0523a538c42d7b42`

## Workspace topology

| Change | Detail |
|--------|--------|
| Workspace root | `Prime/Cargo.toml` (members: Module 1 crates/apps + `Module 2/crates/talos-intake`) |
| Removed | `Prime/Module 1/Cargo.toml` (replaced by `CARGO_WORKSPACE.md`) |
| Removed | Symlink `Prime/Module 1/crates/talos-intake` (Module 2 source only) |
| Lockfile | `Prime/Cargo.lock` (old `Module 1/Cargo.lock` removed) |
| CI | `.github/workflows/ci.yml` `working-directory: Prime` |
| Docs | Root / Prime / Module 2 READMEs use `cd Prime && cargo test...` |

## BatchOutcome

`ImportResult { manifest, outcome }` where `outcome: BatchOutcome`:

| Outcome | Meaning |
|---------|---------|
| `Complete` | `failed_sink == 0`, no rejected frames, `reconcile_ok`; `accepted >= 1` or empty allowed when `fail_on_empty_batch = false` |
| `PartialFailure` | Batch finished with `Ok` but `failed_sink > 0` and/or frame-level rejections |
| `Rejected` | Prefer `Err(TalosError)` for empty batches when `fail_on_empty_batch` (default). `Rejected` is reserved if an `Ok` zero-accepted shape is ever returned |

Config: `fail_batch_on_sink_errors` (default **false**). When `false` and `failed_sink > 0`, still return `Ok` with `PartialFailure`. When `true`, return `Err`.

## Manifest accounting (`BatchCounts`)

| Field | Meaning |
|-------|---------|
| `filesystem_entries_seen` | Files under batch root (not directories) |
| `image_candidates_discovered` | Was `discovered`; serde `alias = "discovered"` |
| `ignored_unsupported` | Non-image files under root |
| `accepted` / `rejected` / `skipped_duplicate` / `failed_sink` / `metadata_warnings` | Unchanged |

`reconcile_ok`:

```text
image_candidates_discovered == accepted + rejected + skipped_duplicate + failed_sink
filesystem_entries_seen == image_candidates_discovered + ignored_unsupported
```

Metadata sidecar: if **outside** root, not walked / not counted. If **inside** root with a non-allowed extension (e.g. `.csv`), counted as `ignored_unsupported`.

## Image validation

1. Magic sniff (JPEG / PNG)
2. `image` decode via `ImageReader` with `max_width`, `max_height`, `max_pixel_count` limits
3. **No** write / re-encode; SHA-256 over original bytes

Truncated / fake-magic decode failures → `RejectedPermanent`. Dimension / pixel-budget violations → `RejectedValidation`.

Defaults in `configs/intake.toml`: `max_width=8192`, `max_height=8192`, `max_pixel_count=25_000_000`, `fail_batch_on_sink_errors=false`.

## Staging lifecycle

Staging under `staging_root/<batch_id>/` is **owned by M02 for intake**. Cleanup is **deferred** to the orchestrator / M10 — successful intake does not remove staging.

## Verification (from `Prime/`)

| Command | Exit |
|---------|------|
| `cargo metadata --no-deps` | 0 |
| `cargo fmt --all -- --check` | 0 |
| `cargo check --workspace --all-targets --all-features` | 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 |
| `cargo test --workspace --all-features` | 0 (44 passed) |
| `git diff --check` | 0 |

## Confirmation checklist

- [x] Symlink `Prime/Module 1/crates/talos-intake` gone
- [x] Workspace root: `Prime/`
- [x] No commit/push from this hardening agent (parent commits)

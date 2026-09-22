# Module 1 — Cargo note

The **canonical Cargo workspace root** is [`../Cargo.toml`](../Cargo.toml) (`Prime/`).

M01 crates and apps live under this module (`crates/`, `apps/`) and are workspace members of `Prime/`.

```bash
cd Prime
cargo test --workspace --all-features
cargo test -p talos-types
```

Do not add a nested `[workspace]` here.

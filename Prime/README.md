# Prime

Prime houses the foundational Talos modules.

## Modules

| Module | Name | Path | Status |
|--------|------|------|--------|
| M01 | Foundation & Core | [Module 1](Module%201/) | Implemented |
| M02 | Batch & File Intake | [Module 2](Module%202/) | **Implemented** (`talos-intake`) |
| M03 | AI Provider Gateway | [Module 3](Module%203/) | **Implemented** (`talos-ai-gateway`) |
| M04 | Image Pre-Processing | [Module 4](Module%204/) | PDR complete |
| M05 | Vision Processing | [Module 5](Module%205/) | PDR complete |
| M06 | OCR & Plate Intelligence | [Module 6](Module%206/) | PDR complete |
| M07 | HSRP Analysis | [Module 7](Module%207/) | PDR complete |
| M08 | Confidence & Decision | [Module 8](Module%208/) | PDR complete |
| M09 | Review Management | [Module 9](Module%209/) | PDR complete |
| M10 | Storage & Data | [Module 10](Module%2010/) | PDR complete |
| M11 | Export & Reporting | [Module 11](Module%2011/) | PDR complete |
| M12 | Auth, Admin & Config | [Module 12](Module%2012/) | PDR complete |

## Develop

Canonical Cargo workspace root is this directory (`Prime/Cargo.toml`):

```bash
cd Prime
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p talos-integration   # G1 gate: M02 intake → M01 port → M03 gateway
```

Cross-module gates live in `Module 1/crates/talos-integration` (test-only, `publish = false`).

## Cross-cutting docs

- [G0 contract freeze report](../docs/G0_CONTRACT_FREEZE_REPORT.md)
- [G1 integration report](../docs/G1_INTEGRATION_REPORT.md)

- [Global ADR registry](ADRs.md)
- [Repository contract audit](../docs/REPOSITORY_CONTRACT_AUDIT.md)
- [M01 contract consolidation](Module%201/docs/architecture/contract-consolidation.md)

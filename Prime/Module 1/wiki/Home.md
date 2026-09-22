# Talos-RS Wiki

**Talos-RS** — Automated Vehicle Identification & HSRP Audit Engine (100% Rust / Tokio).

This wiki covers Module **M01 Foundation & Core** and the hexagonal architecture (M01–M12).

## Quick links

| Page | Description |
|------|-------------|
| [[Getting-Started]] | Clone, test, run fixture spine |
| [[Architecture]] | Hex module map M01–M12 |
| [[M01-Foundation-and-Core]] | Full M01 PDR (normative) |
| [[Pipeline-Stages]] | Stages 0–7 contract |
| [[Configuration]] | TOML + `TALOS__*` env |
| [[ADRs]] | Architecture decision records |

## Product posture (current generation)

- **API-first**, batch / file intake (folders, ZIP, CSV/XLSX)
- **No** live camera access, **no** VAHAN, **no** local GPU by default
- Vision / OCR / HSRP via external **AI Provider Gateway (M03)**
- Decisions are **evidence-driven**, not OCR alone

## Repository

Source: [anutrk111/M1](https://github.com/anutrk111/M1)

In-repo mirror of these pages: [`wiki/`](https://github.com/anutrk111/M1/tree/main/wiki)

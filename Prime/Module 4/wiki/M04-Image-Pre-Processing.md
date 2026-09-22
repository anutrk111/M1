# M04 Image Pre-Processing

Full text: [PDR](../docs/pdr/M04-image-pre-processing.md).

## Owns

- **Stage 0** — quality as evidence (`Accept` / `Degraded` / `Unusable`)
- **Stage 2** — `RectificationResult` = `Plates(Vec<…>)` | `NoPlateDetected`

## Contracts

- Original frame immutable (M02 SHA-256 authoritative)
- `top_k` default 1; N-plate ready
- Three refs: original crop, enhanced, API-prepared
- Transform provenance + derived hashes
- No OCR character invention
- Prefer plate crops to M03 for plate-only ops

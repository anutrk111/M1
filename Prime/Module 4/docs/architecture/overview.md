# M04 Architecture Overview

**M04 Image Pre-Processing** owns Stage 0 (quality) and Stage 2 (rectify/enhance).

## Principle

> Prepare pixels; do not assign meaning. Original evidence never changes.

```text
M02 ORIGINAL FRAME (immutable SHA-256)
        │
        ▼
 M04 Stage 0 — Quality → Accept | Degraded | Unusable
        │
        ▼
 M05 Stage 1 — Detection (via M03) → 0..N plates
        │
        ▼
 M04 Stage 2 — Rectify/Enhance → Vec<RectifiedPlate> | NoPlateDetected
        │
        ├─► M06 OCR (via M03)   ← prefer plate / api-prepared
        └─► M07 HSRP (via M03)
```

## Normative docs

- [PDR M04](../pdr/M04-image-pre-processing.md)
- [ADR 0008 — Quality classes](../adr/0008-stage0-quality-gates.md)
- [ADR 0009 — Multi-plate rectify](../adr/0009-rectify-after-detection.md)

## Related

- [Module 1](../../../Module%201/) · [Module 2](../../../Module%202/) · [Module 3](../../../Module%203/)

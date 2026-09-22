# M05 Architecture Overview

**M05 Vision Processing** owns Stage 1: vehicle + plate detection on the original frame via M03.

## Principle

> Locate objects in the original frame; do not read characters or decide outcomes.

## Lineage

```text
M02 OriginalFrame
 → M04 S0 Quality (Accept/Degraded)
 → M05 S1 Detection (Talos DetectionId + pixel bbox)
 → M04 S2 Rectify (SAME DetectionId)
 → M06 OCR + M07 HSRP
 → M08 Decision
```

## Docs

- [PDR M05](../pdr/M05-vision-processing.md)
- [ADR 0010](../adr/0010-detection-via-m03-only.md)
- [ADR 0011](../adr/0011-multi-plate-detection-ids.md)

## Related

[Module 1](../../../Module%201/) · [Module 3](../../../Module%203/) · [Module 4](../../../Module%204/)

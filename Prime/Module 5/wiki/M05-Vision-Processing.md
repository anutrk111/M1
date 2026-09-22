# M05 Vision Processing

Full text: [PDR](../docs/pdr/M05-vision-processing.md).

## Owns

Stage 1 — vehicle + plate detection on **original frame**.

## Locked rules

- Talos-owned `DetectionId` (provider id optional)
- Canonical pixel `BoundingBox`
- Multi-plate 0..N; empty success ≠ error
- In-frame IoU suppression ≠ Stage 6 dedup
- Auditable `DetectionSummary`
- No plate text / HSRP / decisions
- Remote path only via M03

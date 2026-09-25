# M05 Vision Processing

Full text: [PDR](../docs/pdr/M05-vision-processing.md).

## Owns

Stage 1 — vehicle + plate detection on **original frame**.

## Locked rules

- Talos-owned `DetectionId` (provider id optional)
- Canonical normalized `[0,1]` `BoundingBox` (ADR-0041)
- Multi-plate 0..N; empty success ≠ error
- In-frame IoU **detection suppression** ≠ Stage 6 observation dedup/clustering
- Auditable `DetectionSummary`
- No plate text / HSRP / decisions
- Remote path only via M03

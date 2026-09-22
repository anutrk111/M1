# M07 Architecture Overview

**M07 HSRP Analysis** owns Stage 5: visual HSRP-related observations via M03.

## Principle

> Observe cues; do not adjudicate legality. Not visible ≠ not present.

## Lineage

```text
M04 RectifiedPlate (DetectionId)
 → M07 S5 HSRP observations (ternary observability)
 → M08 Decision (same DetectionId; fusion may use cues, not M07 verdicts)
```

## Docs

- [PDR M07](../pdr/M07-hsrp-analysis.md)
- [ADR 0014](../adr/0014-hsrp-via-m03-evidence-only.md)
- [ADR 0015](../adr/0015-hsrp-bound-to-detection-id.md)
- [ADR 0020](../adr/0020-hsrp-observability-ternary.md)

## Related

[Module 1](../../../Module%201/) · [Module 3](../../../Module%203/) · [Module 4](../../../Module%204/) · [Module 6](../../../Module%206/) · [Module 8](../../../Module%208/)

# M08 Architecture Overview

**M08 Confidence & Decision** owns Stage 6 (observation dedup clusters) and Stage 7 (eligibility → rank → fusion → outcome).

## Principle

> Fuse evidence with hard constraints first; keep every candidate and DetectionId; never overclaim vehicle identity.

## Lineage

```text
M06 GrammarResult + M07 HsrpEvidence (DetectionId)
 → M08 S6 Observation clusters (≠ M05 suppression)
 → M08 S7 Eligibility → rank → outcome
 → optional M03 VLM (additive VisualObservation)
```

## Docs

- [PDR M08](../pdr/M08-confidence-decision.md)
- [ADR 0016](../adr/0016-config-driven-thresholds.md)
- [ADR 0017](../adr/0017-vlm-selective-additive.md)
- [ADR 0018](../adr/0018-no-plate-vs-hard-fail.md)
- [ADR 0021](../adr/0021-eligibility-then-rank.md)
- [ADR 0022](../adr/0022-observation-cluster-not-vehicle.md)

## Related

[Module 1](../../../Module%201/) · [Module 3](../../../Module%203/) · [Module 5](../../../Module%205/) · [Module 6](../../../Module%206/) · [Module 7](../../../Module%207/)

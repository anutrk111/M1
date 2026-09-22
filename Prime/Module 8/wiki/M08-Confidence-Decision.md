# M08 Confidence & Decision

Full text: [PDR](../docs/pdr/M08-confidence-decision.md).

## Owns

- Stage 6 — observation deduplication / clusters (≠ M05)
- Stage 7 — evidence fusion + outcomes

## Locked rules

- Eligibility / hard constraints before primary ranking
- Full candidate set in provenance (siblings not discarded)
- `0.90` / `0.80` = starting config defaults only
- VLM selective, additive; cannot bypass thresholds or erase hard evidence
- Stage 6 = duplicate observation clusters — not “same vehicle”
- DetectionId never lost on decision-facing artifacts
- Outcomes: AutoApproved | SecondaryVerification | ReviewRequired

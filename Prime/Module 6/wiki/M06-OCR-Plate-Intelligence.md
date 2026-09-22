# M06 OCR & Plate Intelligence

Full text: [PDR](../docs/pdr/M06-ocr-plate-intelligence.md).

## Owns

- Stage 3 — OCR via M03
- Stage 4 — Indian registration grammar (local)

## Locked rules

- Hypothesis is structured (DetectionId, source artifact, provenance, raw/normalized text, confidence, optional char evidence)
- Grammar never bypassed on high OCR confidence
- Corrections (e.g. `O↔0`) only with OCR evidence support
- GrammarStatus: `VALID` | `KNOWN_INVALID` | `UNKNOWN_TO_REGISTRY`
- Same DetectionId through GrammarResult
- NoPlateDetected → skip; do not invent text
- No final decisions / HSRP legal verdicts

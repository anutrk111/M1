# M06 Architecture Overview

**M06 OCR & Plate Intelligence** owns Stage 3 (OCR via M03) and Stage 4 (local Indian registration grammar).

## Principle

> Read characters with provenance; validate with evidence-supported grammar; never invent a plate or drop DetectionId.

## Lineage

```text
M04 RectifiedPlate (DetectionId)
 → M06 S3 OCR (structured hypotheses)
 → M06 S4 Grammar (VALID | KNOWN_INVALID | UNKNOWN_TO_REGISTRY)
 → M08 Decision (same DetectionId)
```

## Docs

- [PDR M06](../pdr/M06-ocr-plate-intelligence.md)
- [ADR 0012](../adr/0012-ocr-via-m03-grammar-local.md)
- [ADR 0013](../adr/0013-ocr-bound-to-detection-id.md)
- [ADR 0019](../adr/0019-evidence-supported-grammar.md)

## Related

[Module 1](../../../Module%201/) · [Module 3](../../../Module%203/) · [Module 4](../../../Module%204/) · [Module 5](../../../Module%205/) · [Module 7](../../../Module%207/) · [Module 8](../../../Module%208/)

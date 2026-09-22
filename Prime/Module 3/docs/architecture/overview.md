# M03 Architecture Overview

**M03 AI Provider Gateway** is the sole egress for external Vision and VLM APIs.

## Position in the hex

```text
M05 Vision ──┐
M06 OCR    ──┼──► M03 AI Provider Gateway ──► External Vision/VLM APIs
M07 HSRP   ──┤
M08 Decision (VLM assist) ──┘
```

Depends on **M01** for types, errors, and evidence rules. Does not perform intake (M02) or stage orchestration (M01 pipeline).

## Normative docs

- [PDR M03](../pdr/M03-ai-provider-gateway.md)
- [ADR 0006 — VLM additive evidence](../adr/0006-vlm-evidence-additive-only.md)
- [ADR 0007 — Provider error mapping](../adr/0007-provider-error-mapping.md)

## Related modules

- [Module 1 — Foundation](../../../Module%201/)
- [Module 2 — Batch Intake](../../../Module%202/)

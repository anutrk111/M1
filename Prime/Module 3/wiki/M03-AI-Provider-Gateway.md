# M03 AI Provider Gateway

Full normative text: [PDR](../docs/pdr/M03-ai-provider-gateway.md).

## Role

Sole egress for Vision/VLM HTTP(S) used by M05/M06/M07 and selective VLM assist (M08).

## Operations

| Op | Consumer | Output |
|----|----------|--------|
| Detect | M05 | `Detections` |
| OCR | M06 | `OcrHypothesis[]` |
| HSRP | M07 | `HsrpEvidence` |
| VLM assist | M08 | `VisualObservation` evidence only |

## Rules

- No fake plates when API keys missing
- VLM cannot override grammar / hard-fail
- Cost events on every billable attempt
- Retries only on `Transient` (see ADR-0007)

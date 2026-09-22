# ADR 0021 — Eligibility Then Rank (Full Candidate Provenance)

## Status

Accepted

## Context

Choosing primary plate as unconditional highest fused score lets a high-confidence grammar-invalid candidate beat a lower-confidence coherent eligible candidate.

## Decision

1. Apply **candidate eligibility / hard constraints first** (e.g. grammar status under `hard_fail_on_grammar`, config flags).
2. **Then** rank among **eligible** candidates (default: highest fused score among eligible).
3. If no eligible candidates remain, follow hard-fail / ReviewRequired policy — do not AutoApprove an ineligible winner.
4. **Never discard** sibling plates; the full candidate set remains in decision provenance.
5. Primary selection is recorded with reasons (eligibility filters applied + ranking key).

## Consequences

- Correctness over naive max-score.
- Review and export can still show all siblings.

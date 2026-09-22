# ADR 0031 — Export Views: Machine / Effective / Both

## Status

Accepted

## Context

Consumers need either the immutable machine output, the human-effective result, or both. Collapsing them into one ambiguous column hides whether a row was human-reviewed.

## Decision

1. Export views are explicit: `machine` | `effective` | `both`.
2. `machine` → immutable MachineDecision only.
3. `effective` → ReviewedResult if present, else MachineDecision fallback.
4. Every effective row MUST include `effective_source = machine | human_review`.
5. `both` emits MachineDecision and ReviewedResult as separate fields/sections — never silently replace machine with human.

## Consequences

- Presentation stays honest.
- Downstream analytics can filter on `effective_source`.

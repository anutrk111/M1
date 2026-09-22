# ADR 0023 — Review Actions From M01 Only; Workflow ≠ FrameTerminalStatus

## Status

Accepted

## Context

M01 already defines `ReviewAction` (`Verify` | `Correct` | `Reject` | `Unreadable`) and `FrameTerminalStatus` for pipeline completion. Review UX needs queue states (claimed / in review) that must not pollute the pipeline terminal enum.

## Decision

1. M09 consumes **only** M01 review actions. No alternate action vocabularies.
2. M09 does **not** invent new `FrameTerminalStatus` values.
3. Review item lifecycle uses **workflow states**: `QUEUED` → `CLAIMED` → `IN_REVIEW` → `COMPLETED` (after Verify/Correct/Reject/Unreadable).
4. These workflow states are distinct from pipeline `FrameTerminalStatus` (`Succeeded`, `Failed*`, `Halted`).

## Consequences

- Audit language stays clear: pipeline vs human review queue.
- Downstream storage indexes both axes without conflation.

# ADR 0029 — Review Claim / Lease Concurrency

## Status

Accepted

## Context

Two officers may open the same review item. Without claim/lease and version checks, last-write-wins silently corrupts audit history.

## Decision

1. Workflow states: `QUEUED` → `CLAIMED` → `IN_REVIEW` → `COMPLETED`.
2. Claiming acquires a **lease** (config TTL) and increments / stamps a **version**.
3. Completing an action (`Verify` / `Correct` / `Reject` / `Unreadable`) requires the expected version; mismatch → conflict (retry / re-claim).
4. Expired leases return the item to `QUEUED` (or equivalent reclaimable state) without inventing a FrameTerminalStatus.
5. These states are **not** `FrameTerminalStatus` (see ADR-0023).

## Consequences

- Concurrent reviews are detectable and recoverable.
- Lease TTL is operational config, not a legal SLA claim.

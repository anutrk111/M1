# M09 Review Management

Full text: [PDR](../docs/pdr/M09-review-management.md).

## Owns

Human review workflow: queue, claim/lease, Verify/Correct/Reject/Unreadable.

## Locked rules

- MachineDecision never overwritten
- ReviewEvent append-only; ReviewedResult is effective state
- Workflow states ≠ FrameTerminalStatus
- Correct: previous/corrected value, DetectionId, officer, timestamp, reason/note
- No DetectionId change / new detection / sibling delete
- Officer audit metadata in ReviewEvent, not evidence payload
- Persist only via M10 ports

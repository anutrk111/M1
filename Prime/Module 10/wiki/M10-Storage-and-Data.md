# M10 Storage & Data

Full text: [PDR](../docs/pdr/M10-storage-and-data.md).

## Owns

Durability, integrity, indexing, retrieval behind storage ports.

## Locked rules

- Hash (`sha256`) = integrity identity; logical key for humans
- Originals write-once
- MachineDecision immutable; ReviewEvent append-only
- ArtifactState: PENDING → … → AVAILABLE; failure → RECONCILIATION_REQUIRED
- No false persistence success
- `automatic_delete = false` until formal policy
- Upstream uses ports only; M10 does not authorize reviewers

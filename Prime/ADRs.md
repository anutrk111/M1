# Talos-RS Global ADR Registry

Canonical Architecture Decision Records across M01–M12.  
**Do not renumber** existing ADRs to force sequential ownership order.

| ADR | Title | Module | Status | Link | Purpose |
|-----|-------|--------|--------|------|---------|
| 0001 | Pipeline Stage Contract | M01 | Accepted | [link](Module%201/docs/adr/0001-pipeline-stage-contract.md) | Stages 0–7 contract; no invented OCR |
| 0002 | Evidence Kind Separation | M01 | Accepted | [link](Module%201/docs/adr/0002-evidence-kind-separation.md) | VisualObservation vs ExternalDatabaseVerification |
| 0003 | No Local GPU Default | M01 | Accepted | [link](Module%201/docs/adr/0003-no-local-gpu-default.md) | External AI first; GPU NotImplemented |
| 0004 | Intake Metadata Join | M02 | Accepted | [link](Module%202/docs/adr/0004-intake-metadata-join.md) | CSV/XLSX join rules |
| 0005 | Intake No Silent Drop | M02 | Accepted | [link](Module%202/docs/adr/0005-intake-no-silent-drop.md) | Fail visibly on bad intake |
| 0006 | VLM Evidence Additive Only | M03 | Accepted | [link](Module%203/docs/adr/0006-vlm-evidence-additive-only.md) | VLM cannot override grammar |
| 0007 | Provider Error Mapping | M03 | Accepted | [link](Module%203/docs/adr/0007-provider-error-mapping.md) | Transient/Validation taxonomy |
| 0008 | Stage 0 Quality Gates | M04 | Accepted | [link](Module%204/docs/adr/0008-stage0-quality-gates.md) | Accept/Degraded/Unusable |
| 0009 | Rectify After Detection | M04 | Accepted | [link](Module%204/docs/adr/0009-rectify-after-detection.md) | Stage 2 after M05; same DetectionId |
| 0010 | Detection Via M03 Only | M05 | Accepted | [link](Module%205/docs/adr/0010-detection-via-m03-only.md) | No ad-hoc HTTP in M05 |
| 0011 | Multi-Plate DetectionIds | M05 | Accepted | [link](Module%205/docs/adr/0011-multi-plate-detection-ids.md) | Talos DetectionId; suppression ≠ Stage 6 |
| 0012 | OCR Via M03; Grammar Local | M06 | Accepted | [link](Module%206/docs/adr/0012-ocr-via-m03-grammar-local.md) | OCR remote; grammar in-process |
| 0013 | OCR Bound to DetectionId | M06 | Accepted | [link](Module%206/docs/adr/0013-ocr-bound-to-detection-id.md) | Structured hypotheses |
| 0014 | HSRP Via M03 Evidence Only | M07 | Accepted | [link](Module%207/docs/adr/0014-hsrp-via-m03-evidence-only.md) | No legal verdict in M07 |
| 0015 | HSRP Bound to DetectionId | M07 | Accepted | [link](Module%207/docs/adr/0015-hsrp-bound-to-detection-id.md) | Observation lineage |
| 0016 | Config-Driven Thresholds | M08 | Accepted | [link](Module%208/docs/adr/0016-config-driven-thresholds.md) | 0.90/0.80 starting defaults |
| 0017 | VLM Selective Additive | M08 | Accepted | [link](Module%208/docs/adr/0017-vlm-selective-additive.md) | Ambiguous band only |
| 0018 | NoPlate vs Hard-Fail | M08 | Accepted | [link](Module%208/docs/adr/0018-no-plate-vs-hard-fail.md) | Typed NoPlate policy |
| 0019 | Evidence-Supported Grammar | M06 | Accepted | [link](Module%206/docs/adr/0019-evidence-supported-grammar.md) | No plate invention |
| 0020 | HSRP Observability Ternary | M07 | Accepted | [link](Module%207/docs/adr/0020-hsrp-observability-ternary.md) | NotObservable ≠ false |
| 0021 | Eligibility Then Rank | M08 | Accepted | [link](Module%208/docs/adr/0021-eligibility-then-rank.md) | Hard constraints before primary |
| 0022 | Observation Cluster ≠ Vehicle | M08 | Accepted | [link](Module%208/docs/adr/0022-observation-cluster-not-vehicle.md) | Stage 6 clusters |
| 0023 | Review Actions M01 Only | M09 | Accepted | [link](Module%209/docs/adr/0023-review-actions-m01-only.md) | Workflow ≠ FrameTerminalStatus |
| 0024 | Immutable MachineDecision | M09 | Accepted | [link](Module%209/docs/adr/0024-review-immutable-machine-decision.md) | Append-only ReviewEvent |
| 0025 | Correct Requires DetectionId | M09 | Accepted | [link](Module%209/docs/adr/0025-correct-requires-detection-id.md) | Full Correct contract |
| 0026 | Postgres System of Record | M10 | Accepted | [link](Module%2010/docs/adr/0026-postgres-system-of-record.md) | PG / Timescale / object store |
| 0027 | Immutable Originals Hash Identity | M10 | Accepted | [link](Module%2010/docs/adr/0027-immutable-originals-hash-identity.md) | Hash = identity |
| 0028 | Storage Ports Not Drivers | M10 | Accepted | [link](Module%2010/docs/adr/0028-storage-ports-not-drivers.md) | Hexagonal ports |
| 0029 | Review Claim Lease | M09 | Accepted | [link](Module%209/docs/adr/0029-review-claim-lease.md) | Concurrency |
| 0030 | Artifact State Reconciliation | M10 | Accepted | [link](Module%2010/docs/adr/0030-artifact-state-reconciliation.md) | No false persistence success |
| 0031 | Export Machine vs Effective | M11 | Accepted | [link](Module%2011/docs/adr/0031-export-machine-vs-effective.md) | Views + effective_source |
| 0032 | Export Via M10 Ports | M11 | Accepted | [link](Module%2011/docs/adr/0032-export-via-m10-ports.md) | Read-only ports |
| 0033 | Export Formats CSV/XLSX | M11 | Accepted | [link](Module%2011/docs/adr/0033-export-formats-csv-xlsx.md) | Formats |
| 0034 | RBAC Roles Normative | M12 | Accepted | [link](Module%2012/docs/adr/0034-rbac-roles-normative.md) | Starter roles |
| 0035 | Secrets Env Only | M12 | Accepted | [link](Module%2012/docs/adr/0035-secrets-env-only.md) | No secrets in git |
| 0036 | MFA and Session Policy | M12 | Accepted | [link](Module%2012/docs/adr/0036-mfa-and-session-policy.md) | Revoke + recovery |
| 0037 | Export Manifest Reproducibility | M11 | Accepted | [link](Module%2011/docs/adr/0037-export-manifest-reproducibility.md) | Manifest + SHA-256 |
| 0038 | Spreadsheet-Safe Serialization | M11 | Accepted | [link](Module%2011/docs/adr/0038-spreadsheet-safe-serialization.md) | No storage mutation |
| 0039 | Permission-Based Authorization | M12 | Accepted | [link](Module%2012/docs/adr/0039-permission-based-authorization.md) | Permissions ≠ role names |
| 0040 | Transactional Config Revision | M12 | Accepted | [link](Module%2012/docs/adr/0040-transactional-config-revision.md) | Atomic activation |
| 0041 | Normalized BoundingBox Canon | M01 | Accepted | [link](Module%201/docs/adr/0041-normalized-bounding-box.md) | f32 [0,1]; pixels derived; supersedes 0011 item 2 |
| 0042 | Shared Contract Freeze (G0) | M01 | Accepted | [link](Module%201/docs/adr/0042-shared-contract-freeze.md) | Frozen talos-types; required DetectionId; schema 2.1 |
| 0043 | Three Operational Roles | M01/M12 | Accepted | [link](Module%201/docs/adr/0043-operational-roles.md) | SuperAdmin / Clerk / ReviewingOfficer; refines 0034 |

## Flags

- **No duplicate ADR numbers** under `*/docs/adr/`.
- **Non-monotonic ownership order:** 0019–0020 (M06/M07) authored after 0016–0018 (M08); intentional — do not renumber.
- **Wiki mirrors:** M01 wiki copies of 0001–0003 are documentation mirrors, not second registry entries.

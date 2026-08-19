---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf4-72ab-919d-d2b2aeba76b8
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — artifacts carry provenance and a digest
- **scope:** `oh.war/artifact/v1`.
- **evidence:** digests computed and bound to deliverables.

### OBL-002 — a derived report is REFUSED as an artifact
- **scope:** a report submitted where an artifact is required.
- **evidence:** a plant and its refusal.

## Gate Adequacy

Not required at `basic`. Asked: digest binding proves an artifact is the one submitted, never that it is correct. Correctness is a gate's job.

## Residual Risk

Identity without correctness, as above — inherent, not a defect here.

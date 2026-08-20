---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd7-eb6b-7cc7-a7d3-bb8ba8239f96
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a Submission cannot assert completion
- **scope:** §51.2.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant asserting success, refused.

### OBL-002 — the four attempt kinds are distinct
- **scope:** §52.1–§52.4.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each classified and none collapsing into another.

### OBL-003 — each attempt keeps its own basis
- **scope:** RQ-034.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** amend forward; the prior attempt still cites the old digest.

## Gate Adequacy

Not required at `basic`; should be `controlled` when executed. Asked: a performer claim can be false, which is precisely why it is a claim and not evidence. That is the design working, not a gap.

## Residual Risk

Repair-versus-restart classification is authored and therefore gameable, like every other authored classification in the system.

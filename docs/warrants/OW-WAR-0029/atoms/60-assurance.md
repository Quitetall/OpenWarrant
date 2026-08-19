---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-707d-b64a-27ffb62e8931
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — allocation preserves the UUID
- **scope:** §91.3 test 17.
- **evidence:** UUID before and after are identical.

### OBL-002 — Source Holder is not transferred
- **scope:** RQ-004, §91.3 test 21.
- **evidence:** post-registration, Git remains Source Holder.

### OBL-003 — offline still works
- **scope:** RQ-070.
- **evidence:** the full local flow with KF unreachable.

### OBL-004 — fabrication is still refused
- **scope:** §91.3 test 20.
- **evidence:** the existing plant still passes.

## Gate Adequacy

Not required at `basic`; `controlled` when executed. Asked: registration proves an identifier was issued, not that the issuer is authoritative. With no Identifier Registry (§101.5), any KF instance can issue one.

## Residual Risk

The Identifier Registry does not exist. Until it does, an allocated enterprise ID means only 'some KF said so'.

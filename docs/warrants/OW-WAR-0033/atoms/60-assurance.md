---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b8-7a06-b2a4-af9f2db3f485
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — nine projections exist
- **scope:** §17.5.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each renders; none returns unimplemented.

### OBL-002 — projections do not affect the contract
- **scope:** §91.1 test 3.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the digest is unchanged across all nine.

### OBL-003 — every committed projection drift-checks
- **scope:** the generated tree.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant per committed view.

## Gate Adequacy

Not required at `basic`. Asked: a projection can be complete, drift-free, and still omit what a particular reader needed. Nothing detects a missing-but-wanted field.

## Residual Risk

Omission is invisible. Only a reader noticing catches it.

---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3eb6-7ac2-a2d2-1fac35901cdd
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance obligations

### OBL-001 - canonical typed effect shape

- **scope:** `EffectRecord` fixtures in `wmd-sim`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** wrong commit phase, payload over 4 KiB, unknown entity, and
  property type/range violations refuse.

### OBL-002 - duplicate and atomic refusal

- **scope:** one bounded `EffectBuffer` and atomic groups up to 256 members.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** duplicate canonical sort key refuses before mutation; invalid
  atomic member rejects every group member and publishes no event.

### OBL-003 - deterministic commit events

- **scope:** WMD H2 effect fixtures at commit `765ab4b`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** insertion-order permutation yields canonical event order and
  `./scripts/ci.sh` passes local matrix.

## Gate adequacy and residual risk

Ordering and refusal plants are required because an always-success commit would
not establish authority. Local receipts are performer observations; independent
verification and human authorization are outstanding. Physics and network
effects remain future packages.

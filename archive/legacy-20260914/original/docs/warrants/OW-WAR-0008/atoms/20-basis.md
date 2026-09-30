---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd0-d818-7a39-bd24-88638a843026
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §24 in full, especially §24.6 (truthful combinations) and §24.7 (core transitions); §91.6 tests 36–41.

## Prerequisites

OW-WAR-0007 resolved — milestones must be typed before state can reference them.

## Assumptions and Unknowns

- **Evidenced premise.** The five axes are fully enumerated in §24 with their
  legal combinations, so this is transcription plus enforcement rather than design.
- **Accepted residual risk.** State is stored where? Phase 1 has no journal and no
  KF. Until OW-WAR-0031, state must be DERIVED from the record rather than stored,
  which means some transitions are unrepresentable until there is somewhere to
  write them.

## Constraints and Invariants

- **Five axes, never one.** §24.6 lists truthful combinations; a single enum
  cannot express "resolved but disputed" or "blocked while authorized".
- **Blocking does not erase phase** (§91.6 test 37).
- **Annulment changes standing, not historical outcome** (§91.6 test 41).
- **An illegal transition fails closed** (§91.6 test 36), it does not clamp to the
  nearest legal state.

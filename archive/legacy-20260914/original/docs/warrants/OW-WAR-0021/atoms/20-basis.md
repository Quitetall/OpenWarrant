---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd6-ebf5-76a1-b0c8-afc3276d6ae1
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §46: independence dimensions, blind verifier input, minimums.

## Prerequisites

OW-WAR-0017 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** §46.1 enumerates the dimensions.
- **Blocking unknown.** With one actor, no verification is independent by any
  dimension. Whether that means controlled work simply cannot resolve locally, or
  resolves with independence recorded as absent, is a decision this Warrant must
  make explicitly. Recording it as absent is honest; treating absence as
  satisfied is the failure.

## Constraints and Invariants

- **Independence is declared per dimension**, not as a boolean.
- **The minimum is per assurance level** (§46.3), so a controlled Warrant needs
  more than a basic one.
- **Absent independence is recorded as absent**, never inferred as present.

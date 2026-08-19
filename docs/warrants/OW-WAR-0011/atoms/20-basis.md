---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd1-b0d3-7157-b2b2-9c2625d43897
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §32 in full, especially §32.7 (Preflight meaning); §91.6 test 38.

## Prerequisites

OW-WAR-0019 and OW-WAR-0020 resolved — Preflight checks gates are askable, so gates must exist first.

## Assumptions and Unknowns

- **Blocking unknown.** §32.7 requires Preflight to exercise "the real actor
  path". With no Katana and no live runtime, the real actor path for most stages
  cannot be exercised locally. Preflight will therefore be PARTIAL until Phase 5,
  and must report which readiness dimensions it could not check rather than
  passing them.

## Constraints and Invariants

- **A dimension that cannot be checked reports UNKNOWN**, which blocks readiness.
  Silently passing an unchecked dimension is how Preflight becomes theatre.
- **Preflight is not a gate run.** It proves askability, not results.

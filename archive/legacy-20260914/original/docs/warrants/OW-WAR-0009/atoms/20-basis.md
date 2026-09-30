---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd0-d818-7d1b-969f-79f172735b78
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §28 (contract revisions), §29 (contract content), Law 5, Law 6; §91.6 tests 38 and 39.

## Prerequisites

OW-WAR-0008 resolved — a revision transition is a state transition.

## Assumptions and Unknowns

- **Blocking unknown.** Authorization requires an authority, and §27.2 forbids
  self-authorization. Locally there is only one actor. Whether a local
  authorization is representable at all, or whether revisions stay `draft` until
  KF exists, is an open design question this Warrant must answer explicitly rather
  than sidestep.

## Constraints and Invariants

- **An authorized revision is immutable** (Law 5). Editing one is not a
  supported operation at any level of the API.
- **Progress cannot amend the contract** (Law 6). The execution record and the
  contract are different objects with different mutability.
- **A prior attempt keeps its basis** (§91.6 test 39). Amending forward must not
  retroactively change what an earlier attempt ran against.
- **`PHASE_1_CONTRACT_REVISION` is deleted, not generalised.** It exists so the
  compiler fails to build when revisions arrive.

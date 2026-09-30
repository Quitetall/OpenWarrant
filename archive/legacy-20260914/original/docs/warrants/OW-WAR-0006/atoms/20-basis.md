---
schema: oh.war/atom/v1
warrant_uuid: 01a01bcf-b0e0-78dd-978f-097d07fe380b
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §19.4 (ADR relation to WAR), §20.3 (no retroactive parent rationale), §21 (supersession and deprecation), §91.4 tests 25 and 28.

## Prerequisites

OW-WAR-0005 resolved. ADR atoms parse (shipped, unauthorized — see Intent).

## Assumptions and Unknowns

- **Evidenced premise.** The existing two ADRs parse and render; adding relation
  fields is additive to a format nothing else consumes yet.
- **Accepted residual risk.** Adopting already-shipped work means this Warrant's
  Progress Log is the only record that the Overview preceded its authorization.

## Constraints and Invariants

- **Supersession preserves, never deletes** (§21.4). A superseded ADR remains
  exportable and readable; only its currency changes.
- **Acyclic** (§91.4 test 25). A supersession cycle means no decision is current,
  and must fail closed rather than pick an arbitrary head.
- **A proposed ADR is not current** (§91.4 test 28) and cannot satisfy a
  prerequisite that requires an accepted decision.

---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd5-a42a-7dc9-819f-8ca614dc87eb
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §43 in full: ownership, Gate Definition, lifecycle, qualification, Gate Binding, reusable gates, subject-owned tests.

## Prerequisites

OW-WAR-0016 resolved — obligations cite gate bindings.

## Assumptions and Unknowns

- **Evidenced premise.** §43 fully specifies the objects.
- **Blocking unknown.** §43.1 gives KF ownership of the Registry, and KF
  integration is OW-WAR-0028. Whether a local registry is authoritative in the
  interim, or explicitly provisional, must be decided rather than defaulted —
  the same question OW-WAR-0009 faces for authorization.

## Constraints and Invariants

- **A gate command is not a gate.** A Definition carries identity, version,
  qualification state, and a declared askability contract.
- **Unqualified cannot be bound** (§43.4). Qualification is what distinguishes a
  gate that has been shown to work from one that has been written down.
- **`gate_binding_digest` is a declared domain** and must be used.

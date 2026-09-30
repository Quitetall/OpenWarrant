---
schema: oh.war/atom/v1
warrant_uuid: 01a021a6-0dfc-7c2c-b61a-06b3aad19004
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§98 Phase 6's exit is "a delivery can close only through bounded,
provenance-preserving proof." This is the exit the whole system exists for, and
it is the one with the least evidence behind it.

OW-WAR-0016 through 0022 delivered obligations, the epistemic classes, adequacy
review, the Gate Registry, gate runs, independence and resolution. Between them
they own §91.10, §91.11 and §91.12 — twenty-seven conformance tests — and not one
is cited anywhere.

No Warrant in this repository has ever been resolved through §56.1's thirteen
requirements. All 49 are `draft`. The resolution machinery is implemented, tested
against itself, and has never closed anything.

Measured on 2026-08-20, and worse than "untested": of twenty alpha types sampled,
**twenty are unreachable from the shipped binaries.** `Admissibility`,
`Independence`, `EvidenceItem`, `GateReceipt`, `ResolutionChecks` and the rest are
referenced by no code in `openwarrant-cli` or `openwarrant-compiler`. §40.7's six
prohibited substitutions are enforced by a function nothing calls.

So this Warrant's first deliverable is not a resolution. It is wiring the
validators into the check path, because a control that is correct in isolation
and never reached in the real code path is the exact failure `xtask` already
warns about in its own module documentation.

## Desired Outcome

One Warrant carried to a real §56.2 resolution: thirteen requirements
verified, gates actually run, evidence classed under §40, and an adequacy review
that executed attacks.

## Scope

§38, §39, §40, §43, §44, §45, §46, §56, §57, and §91.10–§91.12 (tests 64–90).

## Non-goals

- No high-assurance controls. Signatures and custody are §98 Phase 9.
- No claim of independence. This repository has one actor, and §27.4 says role
  separation by one person is not organizational independence.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-053`, `WAR-SAS-RQ-054`, `WAR-SAS-RQ-055`, `WAR-SAS-RQ-057`,
  `WAR-SAS-RQ-058` — Complete; §39, §44, §46 and §56 govern.
- Discharges §98 Phase 6 exit and §99 criteria 18, 19, 20 and 22.

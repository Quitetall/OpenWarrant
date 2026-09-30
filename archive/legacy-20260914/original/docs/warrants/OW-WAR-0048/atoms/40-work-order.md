---
schema: oh.war/atom/v1
warrant_uuid: 01a021a7-8436-7aff-a005-a43eeea25886
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. A CI job running the canonical-IR comparison on both target
   triples, with byte comparison across hosts.
2. §91.1 test 1 discharged for real, and OW-WAR-0003's and OW-WAR-0005's recorded
   caveats updated to point at it.
3. A Liminal profile compiled over §82.2's versioned protocol.
4. A §82.3 parity run over the whole compatibility corpus with declared
   observables.
5. `ci.yml`'s runner-tier comment corrected, and the self-hosted plan abandoned
   on the security grounds the comment itself names.

## Frozen Surfaces

§82.3's parity contract and the canonical JSON form under comparison.

## Premade Instructions

- Take the two-host run FIRST. It is free, it is the oldest unmet
  claim in the repository, and it does not depend on Liminal existing.
- Declare the observables before running parity. Writing them afterwards is not
  a parity test.
- Do not cut over on a passing parity run alone. §82.4 says qualified, and one
  green run is not a qualification.

## Autonomy and Escalation

Tier T2 — cutover changes which compiler is authoritative and escalates.

## Rollback

Revert. The Markdown adapter remains the production compiler, which it is today.

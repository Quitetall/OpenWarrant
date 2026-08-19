---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd5-a42a-766a-916c-656c6b37998b
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `GateRun` with execution status, optional verdict, and a receipt digested
   under `oh.war/gate-run/v1`.
2. §44.2's six execution statuses and §96.4's ten migration classes,
   neither collapsed into the other, with a total mapping between them.
3. Askability determined before execution.
4. Required-unknown blocking at resolution.
5. Invalidation propagation to dependent resolutions.
6. Mutating-gate quarantine.

## Frozen Surfaces

§44.2's six execution statuses, §96.4's ten migration classes, and the askability/verdict separation. §96.4 requires migration to preserve its ten exactly.

## Premade Instructions

- An unaskable gate must not pass. The mechanism is §44.5's three-way
  conjunction, NOT the absence of a `Verdict::Unknown` variant — §44.3 defines
  `unknown` and §44.4's own examples record it, so deleting it would make the
  specification's examples unrepresentable. The hazard the first draft named is
  real and is answered by never consulting the verdict alone (OW-ADR-0006).
- The required-pass check is verified over ALL 36 askability × status × verdict
  triples, not over examples. A claim about a space tested on three cases is a
  claim about three cases.
- Askability is decided before any process is spawned. After a spawn, the code
  must not be able to reach `missing_tool`.
- Each reason code that can be reached locally gets a plant, and `missing_tool`
  and `failed` must land on different rules. This is where the parent project
  lost 51 gates.

## Autonomy and Escalation

Tier T1 throughout.

## Rollback

Revert. Gates return to being unrunnable definitions, which is at least honest about producing no results.

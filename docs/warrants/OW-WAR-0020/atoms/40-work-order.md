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
2. All ten execution statuses, none collapsible.
3. Askability determined before execution.
4. Required-unknown blocking at resolution.
5. Invalidation propagation to dependent resolutions.
6. Mutating-gate quarantine.

## Frozen Surfaces

The ten execution statuses and the askability/verdict separation. §96.4 requires migration to preserve them exactly.

## Premade Instructions

- Model the verdict as absent when unaskable. An `Option<Verdict>` where `None`
  means 'not asked' is the whole design; a `Verdict::Unknown` variant would let a
  caller treat it as a result.
- Every one of the ten statuses gets a plant. This is where the parent project
  lost 51 gates.

## Autonomy and Escalation

Tier T1 throughout.

## Rollback

Revert. Gates return to being unrunnable definitions, which is at least honest about producing no results.

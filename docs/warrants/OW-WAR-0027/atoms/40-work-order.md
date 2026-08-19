---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-714f-973c-6decbf23e65d
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. WAR stage graph → BLUT PlanSpec lowering.
2. Resource and artifact mapping.
3. Lineage receipt consumption.
4. An integration test against the local BLUT checkout.

## Frozen Surfaces

The lowering contract. It is a wire format between two systems.

## Premade Instructions

- Test against the real BLUT at `training/engine`, not a mock. It is present; using a mock would waste the one adapter that can be genuinely verified pre-beta.

## Autonomy and Escalation

Tier T1 for the boundary; T2 for the mapping.

## Rollback

Revert. Computational stages remain unlowerable.

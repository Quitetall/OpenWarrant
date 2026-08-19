---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd6-ebf5-77d8-b498-a397823907f3
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `Dispatch` per §47.1 with basis, capabilities, resources, outputs, stop
   conditions.
2. Compilation from a stage plus its Warrant.
3. Actor-specific projection (§47.3).
4. `war dispatch` emitting one.

## Frozen Surfaces

The Dispatch schema. It is the wire format between OpenWarrant and every runtime.

## Premade Instructions

- Default to denial for capabilities (§55.2). An unlisted capability is refused,
  never inherited.
- A Dispatch that references anything outside itself is incomplete; test it by
  compiling one and reading it with no repository present.

## Autonomy and Escalation

Tier T1 — the Dispatch is protocol surface.

## Rollback

Revert. Stages remain undispatchable.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0935b-6870-73d1-bbbb-ca87591adbd4
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `war run` executes STAGE-001 (`ops.echo`) and STAGE-002 (the battery), each
   leaving `dispatches/<id>.json`, a gate run with its receipt bound to the
   dispatch digest, and `submissions/<id>.json`.

## Frozen Surfaces

The gates' definitions; `war run` runs them, it does not edit them.

## Premade Instructions

- A submission requests `verify` on a pass and `block` on anything else.
- Never write a submission by hand for a run that did not happen.

## Autonomy and Escalation

Tier T2.

## Rollback

Remove the run records; nothing else depends on them.

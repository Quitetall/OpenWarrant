---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b8-7086-ae76-38561d173119
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. JSON Schema generation from the Rust types.
2. Schema pack assembly with a transitive digest.
3. Version enforcement against `FormatBasis`.
4. Generated TypeScript types for KF.
5. A drift check: generated schemas match the types.

## Frozen Surfaces

The schema pack identity and version. Consumers pin it.

## Premade Instructions

- The pack is generated and drift-checked like every other projection. A hand-edited schema is the same defect class as a hand-edited parent.

## Autonomy and Escalation

Tier T1 — the pack is the published contract.

## Rollback

Revert. `FormatBasis` returns to naming a pack that does not exist, which is the current state and should be recorded as a known gap either way.

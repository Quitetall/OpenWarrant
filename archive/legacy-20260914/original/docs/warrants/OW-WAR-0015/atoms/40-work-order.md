---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf4-72ab-919d-d2b2aeba76b8
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `Deliverable` typed and bound to the contract.
2. `Artifact` with `artifact_digest` under its declared domain.
3. Derived-report typing, distinct from artifacts.
4. Deliverable-to-artifact binding checked at resolution.

## Frozen Surfaces

The artifact digest domain and what it covers.

## Premade Instructions

- A derived report that claims to be an artifact is refused at parse time.

## Autonomy and Escalation

Tier T2, except the derived-report distinction which is T1.

## Rollback

Revert. Deliverables return to prose.

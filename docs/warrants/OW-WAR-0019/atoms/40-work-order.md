---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd5-a42a-7dc9-819f-8ca614dc87eb
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `GateDefinition` with identity, version, and lifecycle state.
2. Qualification records (§43.4).
3. `GateBinding` attaching a qualified definition to a subject, digested under
   `oh.war/gate-binding/v1`.
4. Refusal of an obligation citing an unbound or unqualified gate.
5. Reusable gate support (§43.6).

## Frozen Surfaces

The Gate Definition and Binding schemas, and the gate-binding digest domain.

## Premade Instructions

- An obligation citing a gate that does not exist must FAIL. That is the exact
  defect the parent project shipped 23 times.
- Qualification is a recorded event, not a boolean somebody sets.

## Autonomy and Escalation

Tier T1 — gates decide completion.

## Rollback

Revert. Obligations return to citing prose.

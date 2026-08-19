---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf4-7b46-8ed4-7e9634593bec
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. Rationale node classes and typed edges (§35).
2. The three assumption classes with blocking semantics.
3. Cycle detection over rationale-support edges (§36.4).
4. `war check` refusing a Warrant with an unresolved blocking unknown at
   authorization.

## Frozen Surfaces

The node classes and edge types. They are what makes rationale queryable rather than readable.

## Premade Instructions

- Reuse the existing three-colour DFS for §36.4; a third cycle detector will
  disagree with the first two.
- A blocking unknown at authorization is an error, not a warning.

## Autonomy and Escalation

Tier T1 — this decides when work may not proceed.

## Rollback

Revert. Rationale returns to prose.

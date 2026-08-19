---
schema: oh.war/atom/v1
warrant_uuid: 01a01bcf-b0e0-78dd-978f-097d07fe380b
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `supersedes` / `superseded_by` relations on `AdrRecord`.
2. Cycle detection over the supersession graph, reusing the three-colour DFS
   already written for Warrant parents.
3. Currency derived from relations, not asserted: an ADR with a `superseded_by`
   is not current regardless of its declared status.
4. The Overview renders the relation graph and flags disagreement between a
   declared status and a derived currency.
5. A Progress Log entry recording the untracked Overview commit by SHA.

## Frozen Surfaces

The `AdrStatus` vocabulary. Adding a state changes what every reader of the Overview is being told.

## Premade Instructions

- Reuse `detect_parent_cycles`'s algorithm rather than writing a second graph
  walk; two cycle detectors will disagree eventually.
- A declared `status: accepted` on an ADR that something supersedes is a
  CONFLICT, reported as an error — not silently overridden in either direction.
- Do not backdate the Overview work. Adopt it explicitly.

## Autonomy and Escalation

Tier T2. The status vocabulary and the conflict-resolution rule escalate.

## Rollback

Revert. The relation fields are additive; ADRs without them keep parsing.

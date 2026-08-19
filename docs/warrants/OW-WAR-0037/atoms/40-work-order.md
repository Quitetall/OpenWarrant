---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7530-adc8-260579bdad3b
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. Structural IR diff.
2. `war diff --from --to`.
3. Contract-digest change attribution.
4. Semantic diff for proposals (§74.2).

## Frozen Surfaces

None. Diff is a read surface over frozen structures.

## Premade Instructions

- Attribute digest movement to a field. 'The digest changed' without a cause is the report that sends people to read both versions anyway.

## Autonomy and Escalation

Tier T2.

## Rollback

Revert. Comparison returns to reading both revisions.

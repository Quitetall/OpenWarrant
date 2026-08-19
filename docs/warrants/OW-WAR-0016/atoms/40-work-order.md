---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf5-7161-9490-5d957bdf0d7d
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `Obligation` per §38.2 with scope kind and disposition.
2. Refusal of an unbounded universal claim.
3. `obligation_refs` resolution from milestones.
4. Resolution aggregation over dispositions (§38.6).
5. Migration of this repository's 21 prose obligations to structured records.

## Frozen Surfaces

The obligation schema and the disposition vocabulary. Resolution reads both.

## Premade Instructions

- Migrate the existing 21 obligations FIRST and let their real shape drive the
  schema. A schema designed without them will not fit them.
- Preserve every bounded-scope statement verbatim during migration; those
  sentences are the honest part.

## Autonomy and Escalation

Tier T1. Obligations decide what 'done' means.

## Rollback

Revert. Obligations return to prose, checked by a substring search.

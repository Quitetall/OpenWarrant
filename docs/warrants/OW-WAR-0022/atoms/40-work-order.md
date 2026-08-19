---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd6-ebf5-7339-94c1-44ae5d81a7df
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `Resolution` binding `contract_digest` and `assurance_case_snapshot_digest`.
2. Falsification as a first-class verdict.
3. Dispute and annulment records preserving history.
4. Judgment-authority checking (RQ-058).
5. Ongoing-validation records (§57), distinct from the completion gate.

## Frozen Surfaces

The resolution record's bound digests. They are what makes a resolution checkable years later.

## Premade Instructions

- Falsification is not an error path. Implement it as a peer of satisfaction or
  it will be bolted on as one.
- Dispute and annulment must be additive records. Nothing overwrites a resolution.

## Autonomy and Escalation

Tier T1.

## Rollback

Revert. Warrants remain uncloseable, which is the current honest state.

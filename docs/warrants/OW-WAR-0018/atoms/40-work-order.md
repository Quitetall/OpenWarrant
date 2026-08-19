---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd4-ab16-7823-bf23-99904a35aea0
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `AdequacyReview` with question, outcomes, executed attacks, limitation.
2. Enforcement at `controlled` and `high` (§39.4).
3. Deletion of the substring check.
4. Migration of this repository's three existing reviews.

## Frozen Surfaces

The review record's required fields. They are what makes the review checkable.

## Premade Instructions

- Delete `text.contains("adequacy")` in the same commit that adds the real check.
  Two checks for one rule means the weak one decides.
- A review whose 'executed attacks' section is empty must be reported — this
  repository's own reviews are currently in that state and must show up.

## Autonomy and Escalation

Tier T1.

## Rollback

Revert. RQ-055 returns to a substring search, which is worse than nothing because it reports PASS.

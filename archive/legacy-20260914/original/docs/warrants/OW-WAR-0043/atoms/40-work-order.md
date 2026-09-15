---
schema: oh.war/atom/v1
warrant_uuid: 01a021a2-b571-794a-acf0-1559844cf662
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. The LamQuant ADR corpus imported at one named, frozen commit.
2. A count of what became a `HistoricalClaim` versus a resolution, with the
   expectation that the resolution count is ZERO.
3. Every `gate_cmd` imported as `legacy_declared_unqualified`.
4. Plants for §91.4 test 24 and §91.5 tests 30–35 against the shipped binary.
5. A list of unmapped elements, with a human disposition for each.

## Frozen Surfaces

§96.2's twelve-row mapping table and §96.4's ten preserved classes.

## Premade Instructions

- The resolution count after import is expected to be ZERO. If it is
  not, something promoted a claim, and that is the defect — not a success.
- Do not tidy LamQuant's ADRs on the way through. A migration that improves the
  prose has destroyed the thing it was migrating.
- §91.5 tests 30–35 are about parent/child honesty. They get plants because the
  corpus being imported has real parent relationships to get wrong.

## Autonomy and Escalation

Tier T2 — unmapped elements escalate to a human rather than being guessed.

## Rollback

Revert the import. The corpus stays in LamQuant, unimported, which is where it is now.

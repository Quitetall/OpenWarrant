---
schema: oh.war/atom/v1
warrant_uuid: 01a021a7-8437-7b0a-871a-e65c0b221555
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. §91.2 test 10 implemented, or OW-WAR-0005's claim narrowed by
   amendment with an ADR recording which and why.
2. A §31 amendment record against OW-WAR-0005 carrying the semantic diff.
3. Tests 11, 13, 14 and 15 implemented or explicitly narrowed, with the
   `check.rs` note updated to match reality either way.
4. OW-WAR-0005 M4's untracked work dispositioned under §95 with a reviewer.
5. Plants for whichever of the five tests end up implemented.

## Frozen Surfaces

Nothing new. This Warrant closes existing surfaces rather than adding any.

## Premade Instructions

- Correct the false claim FIRST. It is the only item here that is
  actively wrong rather than merely incomplete, and it sits in a resolved
  Warrant where a reader would trust it.
- Do not edit OW-WAR-0005's obligation text directly. The amendment is the
  record that it was wrong, and deleting the evidence of a defect while fixing it
  is the habit this project exists to break.
- If a test is narrowed rather than implemented, the narrowing must appear in the
  obligation's scope, not only in a commit message.

## Autonomy and Escalation

Tier T2 — amending a resolved Warrant's claim is a manual revision under §30.3.

## Rollback

Revert the amendment. The gaps return to being recorded and open, which is where they are now.

---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b8-7a06-b2a4-af9f2db3f485
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. The seven remaining views.
2. `war show --view <name>` for each.
3. Drift checks for any committed projection.
4. Appendix B conformance for the Work Order view.

## Frozen Surfaces

The projection names in §17.5. Callers request by name.

## Premade Instructions

- Every projection gets a drift check on the day it is committed, not later.
- If a projection needs data the IR lacks, stop and fix the IR's Warrant.

## Autonomy and Escalation

Tier T2.

## Rollback

Revert. Callers get the explicit unimplemented error they get today.

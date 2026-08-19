---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a1f-771a-a8ed-983d14257cfe
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. The §94 measurement set, journalled.
2. Derived metrics per §94.
3. WAR identity on commits and artifacts (§95).
4. Untracked-work detection over the repository history.
5. A first run against this repository, which will find `3678455`.

## Frozen Surfaces

The measurement names. Tuning decisions will cite them.

## Premade Instructions

- Run it against this repository's own history first. It must find the ADR
  Overview commit; if it does not, the detector does not work.
- Never auto-attach an untracked commit to a Warrant.

## Autonomy and Escalation

Tier T2, except the no-fabrication rule which is T1.

## Rollback

Revert. Untracked work goes back to being found by reading the log.

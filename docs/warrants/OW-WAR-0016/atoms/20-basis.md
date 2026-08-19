---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf5-7161-9490-5d957bdf0d7d
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §38 in full: obligations not one prose claim, obligation schema, scope kinds, universal claims, dispositions, resolution aggregation.

## Prerequisites

OW-WAR-0007 resolved — obligations are referenced by milestones via `obligation_refs`, which currently dangle unchecked.

## Assumptions and Unknowns

- **Evidenced premise.** §38.2 gives the schema and this repository already has
  21 obligations written to it, so there is a real corpus to validate against.
- **Accepted residual risk.** Converting existing prose obligations to structured
  records is a migration; a careless one loses the bounded-scope language that
  makes them honest.

## Constraints and Invariants

- **An obligation declares its scope** (§38.4). An unbounded universal claim is
  refused, because 'it works' is not checkable.
- **A completion summary is never one claim** (§38.1). Aggregation is over
  dispositions, and a single overall verdict with no dispositions is refused.
- **`obligation_refs` must resolve.** They dangle today.

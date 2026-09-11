---
schema: oh.war/atom/v1
warrant_uuid: 01a09274-ad79-7414-a583-cdecca67ed90
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

- Governing source: `docs/sas/WAR_Software_Architecture_Specification.md`.
- §36.2 accepted residual risk; §36.3 blocking unknown, which already carries
  `resolution_requirement`; §36.4 prohibition on circular validation.
- §6.10 — levels, and the rule that a Warrant is one bounded intervention.
- The approved 1.0 plan, slice D1: `WarrantStatus` gains
  `unknowns[{id, statement, resolution_requirement, external_dependency, ref}]`
  and `Assumption` gains optional `external_dependency`. This Warrant extends
  the same type and must land compatibly with D1, not beside it.
- Branch base: `feat/battery-split` at `77953b9`.

## Evidence from a consuming program

The WeaponsOfMageDestruction program maintains an ADR gap matrix by hand. A row
reads:

> `| 12 | Body/damage/armor | Future ADR-0018 | Draft and accept before WP-020 |`

The decision is numbered and bound to the package it blocks before it exists.
That table is what makes "which unblock is cheapest" answerable in one read, and
it is how an accepted decision was identified as gating an entire release wave.

It is also entirely unchecked. Its correctness depends on a human editing it,
and nothing in that program detects a work package that requires a decision for
which no row was ever written.

## Prerequisites

None blocking. Ordering constraint rather than prerequisite: slice D1 also edits
`Assumption`. Whichever lands second rebases; both changes are additive with no
`deny_unknown_fields`, so they compose.

## Blocking unknowns

- U-001: whether extending a core type shared with the D1 projection is an
  architecture-changing revision under §101.3 requiring an ADR. Narrowed: the
  reviewing session determined that it is — extending a core record type changes
  what a record may say. The ADR deliverable is retained. This is a reader
  determination recorded as such, not a disposition; §101.3 remains the owner's
  to apply at acceptance.
- U-002: resolved. The shape of `bound_to` was decided by the session that owns
  the D1 Gaps view, which is where the field's value is consumed. It is a list
  of refs, minimum one, each either `roadmap://<NS>-PHASE-N/<slug>` or
  `war://<uuid>#<STAGE-id>` — stage-precise where a stage exists, phase-level
  where the work is not yet warranted. The Gaps view groups by
  `external_dependency` and sorts by the earliest `bound_to`.

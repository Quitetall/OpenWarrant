---
schema: oh.war/atom/v1
warrant_uuid: 01a0746d-d84b-7401-bc57-fb7d6304aeb1
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

OpenWarrant's roadmap parser embeds phases 0 through 10, while Liminal's
existing program runs from Phase -1 through Phase 12. Status also guesses a
namespace from Warrants and substitutes OpenWarrant's phase table when another
program's SAS does not contain exactly eleven phases. Those substitutions
silently change the selected program and cannot support Liminal's SAS migration.

## Desired Outcome

Canonical signed phase references resolve only against the selected program's
SAS declarations, bound to that revision's source and digest. Unavailable
authority stays explicit. Draft inspection remains labelled draft.

## Scope

Roadmap reference grammar, declaration extraction, repository selection,
traceability checks, status projection, and positive/refusal fixtures.

## Non-goals

No Liminal compiler adapter, canonical IR change, program renumbering, SAS
acceptance, Warrant authorization, self-verification, resolution, or corpus
cutover. OW-WAR-0040 remains separate and unchanged.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0cc13-dd90-71b1-9fc5-9989932b3bae
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

To learn what is authoritative in this repository right now, a reader opens
four generated documents and follows `supersedes` by hand: `CORPUS_STATUS.md`
lists 108 Warrants with superseded ones in the same table as current ones,
`WARRANT_OVERVIEW.md` has a currency column, `ADR_OVERVIEW.md` shows proposed
and accepted decisions side by side, and `NORMATIVE.md` holds the SAS in
force. The answer is derivable and nobody derives it; the owner's words on
2026-09-22 were "having to read 100 docs that supersede each other and refer
to the next".

Two defects in how currency and authorship work today make the fix more than
a fifth overview:

- Currency is a **field**. OW-WAR-0073 was marked `currency = "superseded"`
  in its manifest when OW-WAR-0112 superseded it, and its signed contract
  digest moved: the manifest's bytes are in the preimage. §21.2 says the
  replaced Warrant's canonical currency *becomes* superseded and that it
  remains immutable — both at once are only possible if currency is
  computed from the successor's relation.
- `war new` writes `TODO`. Every one of 108 Warrants answered the same five
  questions from a blank heading, in 108 shapes. §8 asks for typed authored
  atoms; nothing here types them.

## Desired Outcome

One canonical **master document**, `docs/generated/CURRENT.md`, generated
by `war compile` and drift-checked, that carries the current software state
and status fully expanded — every current subject's atoms verbatim in role
order, the SAS in force by its normative statements, accepted ADRs, who
governs each path, who may sign, and the queue with every signing command
already judged by the dry run — and points at everything it replaced by one
line of lineage each. One **optional** `docs/generated/HISTORY.md` that
holds all of it, including what was replaced. Currency computed from
`supersedes`; a written `currency = "superseded"` refused. OW-WAR-0073's
signature restored by reverting its mark. `war new --preset` writing typed
atoms with the questions already asked, and `war check` naming a question
left unanswered. Only authored atoms are ever edited; their role is their
relation to the projections; a role nothing renders is refused.

## Non-goals

- Atomizing the SAS document itself. `NORMATIVE.md` already extracts its
  statements; the prose stays one file under §101.
- Retiring `CORPUS_STATUS.md`, `WARRANT_OVERVIEW.md`, `ADR_OVERVIEW.md` or
  their JSON forms: the app and `war status` read them. They stop being the
  first thing a person reads.
- A source-map editor for projections (§17.4). The CLI keeps refusing
  direct parent edits, as §17.4 permits a minimal v1 to.
- Migrating the 108 existing Warrants onto presets. Presets are for what is
  written next; a bound atom is not rewritten to match a template.

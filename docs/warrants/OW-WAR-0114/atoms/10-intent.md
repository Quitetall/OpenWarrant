---
schema: oh.war/atom/v1
warrant_uuid: 01a0cd26-1b69-7123-a9fe-da2516447ada
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Asked on 2026-09-23 whether closing every existing Warrant would deliver the
software the owner wants, the honest answer was no, and the reason was that
the plan is not written down in one place anyone can check.

The repository has four roadmaps and none of them is a record:

- `roadmap://` refs on Warrants, checked for spelling and never resolved.
- SAS §98, which holds the phases and so turns every re-plan into a SAS
  revision (against §6.3's SHALL NOT).
- `docs/roadmap/view.json`, read by one command from a hard-coded path.
- `PRODUCTION_ROADMAP.md`, prose that code reads only to count one word.

Two more plans sit in drafts: the four-phase SDK plan and
`rc2-implementation-roadmap.json`. 40 of 110 Warrants name no phase. The
gaps found that day are assigned nowhere:

- six unaddressed §106 rows;
- Phase 10;
- independent verification;
- batch signing;
- retention;
- teams;
- measuring the friction targets;
- ten engineering contracts the product spec says are still to specify.

## Desired Outcome

One roadmap per program, as a record beside the SAS:

- `docs/roadmap/` holds a manifest and two atoms (intent, and typed phases:
  `id`, `title`, `outcome`, `exit`, `depends_on`, `priority`). Revisions are
  accepted by a human signature that costs one dialog.
- Membership comes from Warrants' refs; achievement is derived.
- `war check` refuses a ref to a phase the roadmap lacks and warns on a
  Warrant with none.
- `war roadmap` shows, assigns and proposes. At a terminal or in the app,
  a human edits phases by keystroke and signs once.
- The master document renders the roadmap.
- The four legacy roadmaps are retired to lineage.

Before any of that is built, this Warrant reconciles:

- every Warrant and every gap is placed under both candidate phase sets
  (the eleven §98 phases, and the four-phase plan);
- the owner picks one with a one-word answer to Q-001;
- the chosen set becomes roadmap revision 1.

## Non-goals

- Cross-Warrant scheduling or a Gantt view. The roadmap orders phases; it
  does not schedule people or dates.
- Changing any Warrant's signed contract to add a ref. A signed Warrant's
  placement is recorded in the roadmap's mapping and applied by amendment
  only when that Warrant is next amended.
- Writing the Warrants for the gaps. Each gap becomes a phase slug marked
  "no Warrant yet"; drafting them is the next work, in the order the
  accepted roadmap gives.

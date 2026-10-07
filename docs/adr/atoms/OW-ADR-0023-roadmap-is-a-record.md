---
schema: oh.war/atom/v1
adr_uuid: 01a0cd26-1b7a-73ba-a8d7-052a823a068e
local_alias: OW-ADR-0023
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0cd26-1b69-7123-a9fe-da2516447ada"
---

# ADR OW-0023: The roadmap is a record — one per program, beside the SAS

## Status

Proposed by the performer under OW-WAR-0114. Adopted when the owner
authorizes that Warrant. Moving §98's phases out of the SAS text is a later
SAS revision with its own acceptance; this ADR does not change a §106 row.

## Context

On 2026-09-23 the repository had four roadmaps and none of them was a
record:

- `roadmap://` refs on Warrants, whose spelling is checked and whose target
  is never looked up. The phase range is a constant, `MAX_PHASE = 10`.
- SAS §98, parsed at run time for titles and Exit criteria, with a stale
  fallback table in `status.rs`.
- `docs/roadmap/view.json`, an authored tree only `war progress` reads, from
  a hard-coded path, and that `war check` never validates.
- `PRODUCTION_ROADMAP.md`, which code reads only to count `**resolved**`.

Two further plans lived in drafts: the four-phase SDK plan (rc.3
`roadmap.json`) and `rc2-implementation-roadmap.json`. 40 of 110 Warrants
named no phase, and the gaps listed on 2026-09-23 were assigned nowhere.

§6.3 separates the Roadmap from the SAS and says scheduling changes SHALL
NOT require architecture changes. With the phases written into §98, every
re-plan was a SAS revision.

The closest thing the tool already models is a Warrant's own milestones
graph: milestones with `depends_on` and exits, and stages that do the
work. A roadmap is that graph one level up. §6.10 says it in the SAS's own
words: "a program has exactly one SAS … its phases are the program's
milestones."

## Decision

1. **One roadmap per program, beside the SAS.** The SAS says what the
   system is. The roadmap says in what order it is built: sequencing,
   dependency, priority and phase (§6.3). A Warrant is one bounded piece.
2. **It is its own subject kind, made of atoms.** `docs/roadmap/roadmap.toml`
   (`oh.war/roadmap/v1`) is the manifest.
   - `atoms/10-intent.md` holds why the program exists and the outcome of
     the whole.
   - `atoms/20-phases.yaml` holds one typed entry per phase: `id`, `title`,
     `outcome`, `exit`, `depends_on`, `priority`.
3. **The roadmap owns the phases.** §98 becomes a pointer to the record, in
   the next SAS revision. After that, re-planning never touches the SAS.
4. **It holds no member list and no status.**
   - Membership is a relation: a Warrant's `[[roadmap]]` ref names its
     phase (OW-ADR-0022, currency by relation).
   - Achievement is derived: a phase is achieved when its `exit`-slugged
     Warrant resolves satisfied.
   - A roadmap is never "resolved".
5. **Revisions are signed, and the signature is cheap.** Each accepted
   revision is recorded under `docs/roadmap/revisions/`, the SAS revision
   flow reused. The cost to a human is one act:
   - **Human-initiated:** `war roadmap` at a terminal, or the app's Roadmap
     pane, edits phases by keystroke. One `war sign roadmap --ssh-sign`
     dialog accepts, showing a phase-level diff. No file is opened and no
     document is read.
   - **Agent-initiated:** `war roadmap propose` writes the atoms. The
     queue shows `accept roadmap rev N+1` with its diff and its dry-run
     verdict: one line, one dialog.
   - **Moving a Warrant between phases needs no roadmap signature.** It
     changes the Warrant's ref, not the roadmap.
6. **The roadmap renders into the master document** (`CURRENT.md`,
   OW-ADR-0022) as its Roadmap section: phases in dependency order, each
   with its exit, members and rungs. Earlier revisions go to `HISTORY.md`.
   No separate roadmap document is presented. The four legacy roadmaps
   become one line of lineage each.

## Why not the alternatives

- **A parent Warrant with a `program` profile.** A Warrant is one outcome
  that resolves. A roadmap has many phases and never finishes. §20.5's
  parent resolution rule would either never fire or fire wrongly.
- **Keep the phases in §98.** This would contradict §6.3's SHALL NOT every
  time the plan moves.
- **An unsigned planning file.** The plan could then shift under signed
  work without anyone agreeing to it. The owner chose signatures, on
  condition that they cost nearly nothing, and they do.
- **A member list on each phase.** That would be a second place to keep the
  fact a Warrant's ref already states, and the one that drifts.

## Consequences

- `roadmap.unknown-phase` replaces the hard-coded phase range: a ref must
  name a phase the accepted roadmap has.
- `roadmap.unassigned` warns on a current Warrant with no ref. Its remedy
  is the automatic `war roadmap assign` while the Warrant is unsigned, and
  an amendment after.
- The eleven §98 phases, the four-phase SDK plan and every 2026-09-23 gap
  are reconciled once, in OW-WAR-0114, into one accepted phase set.
- A program without a roadmap record keeps today's behaviour (§98 parsed)
  until it adopts one.

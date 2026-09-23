---
schema: oh.war/atom/v1
warrant_uuid: 01a0cd26-1b69-7123-a9fe-da2516447ada
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Scope

A new subject kind (the roadmap record), its checks, its command, its
acceptance through the existing signing seam, its low-friction editor, and
its rendering in the master document. Plus the one-time reconciliation that
fills revision 1.

## Deliverables

M1 — reconciliation:

1. The mapping in `20-basis.md`: every Warrant and every gap under both
   candidate phase sets. The owner answers **Q-001** (blocking, on
   STAGE-001) with `A` or `B`, one word.
2. `docs/roadmap/roadmap.toml`, `docs/roadmap/atoms/10-intent.md` and
   `docs/roadmap/atoms/20-phases.yaml`, filled from the chosen set.
   - Every phase gets its Exit (A: verbatim from §98; B: the four rewritten)
     and its `priority`.
   - Every gap becomes a phase slug, listed under that phase's `open` with
     "no Warrant yet".

M2 — the record:

3. `crates/openwarrant-core/src/roadmap.rs`: `Roadmap`, `Phase`,
   `RoadmapRevision`.
   - Parsing refuses duplicate ids, an unknown `depends_on`, and a cycle.
   - `schemas/oh.war/roadmap/v1.json` is regenerated; the pack version moves
     once.
4. Repository loader methods: `load_roadmap`, `load_roadmap_revisions`.
5. `crates/openwarrant-cli/src/roadmap_cmd.rs`:
   - `war roadmap` shows phases, members, rungs and achievement.
   - `war roadmap assign <alias> <phase>` writes the ref on an **unsigned**
     Warrant and refuses a signed one, naming the amendment path.
   - `war roadmap propose` records the atoms' digest as a proposed revision.
   - `--json` everywhere.
6. `check.rs` rules:
   - `roadmap.unknown-phase` (error);
   - `roadmap.unassigned` (warn; auto remedy `war roadmap assign`);
   - `roadmap.cycle` (error);
   - `roadmap.unaccepted` (warn; human remedy `war sign roadmap --ssh-sign`).
   - `traceability.rs`: the phase range comes from the record when one
     exists; `MAX_PHASE` only without one.
7. `status.rs` (cli and core): Objectives come from the accepted roadmap's
   phases, with the existing achievement logic unchanged. The stale
   `PHASES` table is kept only as the no-record fallback, corrected to 1.1.0's
   eleven Exits.

M3 — acceptance:

8. `sign.rs`: `Pending::AcceptRoadmap { revision }`, drafted, ingested and
   attested through the SAS acceptance path. `--dry-run` works as for every
   act. The signing screen shows a phase-level diff: added, removed,
   retitled, re-exited, re-ordered.

M4 — low-friction editing:

9. `crates/openwarrant-cli/src/roadmap_edit.rs`: a pure state machine in the
   `init/guided.rs` pattern.
   - Edits: add, rename, reorder, set exit, set priority, remove (refused
     while members remain).
   - Front ends: a line front end behind `sign::at_a_terminal()`, and the
     app's **Roadmap** pane (`tui/mod.rs`, its tenth pane).
   - Each ends with one `war sign roadmap --ssh-sign` child.
   - No file is opened by the human; `docs/TUI.md` documents the pane.

M5 — rendering and retirement (after OW-WAR-0113 M2):

10. `current.rs`: the Roadmap section of `CURRENT.md`. `HISTORY.md` carries
    earlier revisions.
11. `progress_viewer.rs` and `progress_viewer/roadmap.rs` read the record;
    `view.json` is retired.
12. `PRODUCTION_ROADMAP.md`, `view.json`, `rc2-implementation-roadmap.json`
    and rc.3 `roadmap.json` are marked retired with one line of lineage
    each.
13. The SAS text: §98 reduced to a pointer at the roadmap record, proposed
    as the next SAS revision (`war sas propose`). Its acceptance is the
    owner's separate act.
14. `CONTEXT.md`: the term *roadmap* as OW-ADR-0023 defines it.
15. `conformance/plants.d/70-roadmap.sh`: the plants listed in OBL-001 to
    OBL-004.

## Frozen Surfaces

`oh.war/report/v1`, every existing record schema, the signing seam's
interface (a new act kind, no new signing path), and every signed Warrant's
manifest.

## Premade Instructions

- Membership is a relation. The roadmap never lists a Warrant; a phase's
  members are computed from refs, like currency.
- A human edit is a keystroke and one dialog. If a step makes the human open
  a file or read a document, it is wrong.
- The dry run judges every roadmap acceptance before it is handed over.

## Autonomy and Escalation

Tier T2. Q-001 (the phase set) is the owner's. Escalate rather than decide:

- whether a phase may be removed while it has resolved members (draft
  says no; history lives in `HISTORY.md`, not in a deleted phase);
- whether `priority` is a number or one of the four named tiers (draft says
  the tiers under A).

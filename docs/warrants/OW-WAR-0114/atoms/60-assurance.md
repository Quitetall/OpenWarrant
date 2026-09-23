---
schema: oh.war/atom/v1
warrant_uuid: 01a0cd26-1b69-7123-a9fe-da2516447ada
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — every Warrant and every gap is placed, under the set the owner chose
- **scope:** `20-basis.md`, `docs/roadmap/atoms/20-phases.yaml`, Q-001.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** Q-001 carries the owner's answer; every current Warrant in the corpus appears in the mapping; every gap row is a phase slug in revision 1; after M2, `war status --json` shows no "unassigned" objective for any unsigned Warrant. A signed one is listed with its mapped phase and the note "applied at next amendment".

### OBL-002 — the roadmap is a record the checker holds Warrants to, and it refuses what it should
- **scope:** `core/roadmap.rs`, `roadmap_cmd.rs`, `check.rs`, `traceability.rs`, `status.rs`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** on a scratch program, a ref to a phase the roadmap lacks is `roadmap.unknown-phase`; a `depends_on` cycle is `roadmap.cycle`; a Warrant with no ref warns `roadmap.unassigned`, and its auto remedy assigns it; `war roadmap assign` on a signed Warrant refuses and names the amendment; removing a phase that has members fires `unknown-phase` on each member by name; on this corpus, Objectives come from the record and match `war roadmap`'s phases one-for-one.

### OBL-003 — changing the roadmap takes one human act, and only a human's
- **scope:** `sign.rs`, `roadmap_edit.rs`, the app's Roadmap pane.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** An edit to the phases atom with no acceptance reads `roadmap.unaccepted`, with remedy `war sign roadmap --ssh-sign`; `war sign roadmap --dry-run` reports `would-record` with the phase diff, and writes nothing; the edit machine, driven with canned answers, writes only `20-phases.yaml` and ends in exactly one signing child; `war roadmap` without a terminal refuses to edit; an agent actor's acceptance is refused by kind; no step in the human flow opens a file.

### OBL-004 — the master document carries the roadmap, and the legacy roadmaps are lineage only
- **scope:** `current.rs`, `progress_viewer`, `docs/roadmap/`, the SAS proposal.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `CURRENT.md`'s Roadmap section lists every phase in dependency order with exit, members and rungs; `war progress` renders from the record with `view.json` absent; `PRODUCTION_ROADMAP.md`, `view.json` and the two draft JSON plans each appear once, as lineage; `war sas status` shows the proposed revision whose §98 points at the record.

## Gate Adequacy

Required at `basic`. The load-bearing obligations are OBL-002 and OBL-003,
because each pairs a claim with the refusal that shows the control exists:

- a record the checker does not hold Warrants to is a fifth roadmap;
- a signature that costs more than one act will be skipped, and an
  unsigned plan is the drift this Warrant removes.

**Adversarial question:** could the roadmap claim a phase is achieved when it
is not? Achievement is derived by the existing `status.rs` logic from the
exit Warrant's recorded resolution. The roadmap holds no status field to
assert it with.

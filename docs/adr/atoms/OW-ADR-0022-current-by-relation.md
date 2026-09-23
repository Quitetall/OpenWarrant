---
schema: oh.war/atom/v1
adr_uuid: 01a0cc14-ce2f-7628-8a3a-ff64a526cd91
local_alias: OW-ADR-0022
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0cc13-dd90-71b1-9fc5-9989932b3bae"
---

# ADR OW-0022: Two projections, current by relation; atoms are the only authored thing

## Status

Proposed by the performer under OW-WAR-0113. Adopted when the owner
authorizes that Warrant; nothing here changes a §106 row, so no SAS revision
carries it (§101.3).

## Context

The repository is built the way SAS §7 (Law 1) says — atoms are authored,
parents are projections — and then stops one level short. To learn what is
authoritative *now* a reader opens four generated files
(`CORPUS_STATUS.md`, `WARRANT_OVERVIEW.md`, `ADR_OVERVIEW.md`,
`NORMATIVE.md`), sees superseded Warrants in the same table as current ones,
and follows `supersedes` by hand. The owner asked for one master document
that is current by construction, and for the number of authored things —
and of "presentable" things — to be as small as the facts allow
(2026-09-22).

Two facts from the corpus shape the decision:

- **Currency written into a manifest moves a signature.** OW-WAR-0073 was
  marked `currency = "superseded"` on 2026-09-22 and its revision-1
  contract digest moved (`manifest_digest` is in the preimage,
  `ir.rs`). §21.2's "canonical currency becomes superseded" must therefore
  be a fact *derived* from the successor's `supersedes` relation, never a
  field edited into the predecessor — the predecessor is immutable (§21.2,
  RQ-084).
- **`war new` writes `TODO`.** Every authored atom in this corpus was
  written from a blank heading; the same five questions are answered in
  different shapes 108 times. §8 wants typed authored atoms rather than
  arbitrary model-written files.

## Decision

1. **Two projections, and only two, are presentable.**
   - `docs/generated/CURRENT.md` — the **master document**: every current
     subject fully expanded, atom by atom in role order, verbatim; the SAS
     revision in force by its normative statements; accepted ADRs; who
     governs each path (the ownership index, OW-ADR-0021); who may sign;
     and the queue, each signing command annotated by the dry run. A
     superseded, annulled or replaced subject appears as **one line of
     lineage** (`OW-WAR-0073 → OW-WAR-0112`) and nothing else, unless a
     current atom references it — and a reference is a link, never text.
   - `docs/generated/HISTORY.md` — **optional**, on by
     `[generated] history = true`: every subject that ever existed,
     lineage chains, superseded atoms expanded, the state timeline. It is
     the audit trail RQ-084 promises, kept out of the master document.
   Every other generated view is an input to these two or a machine
   projection (JSON); none is a document a person is asked to read first.
2. **Currency is a relation, not a field.** `currency(S)` is *superseded*
   iff an authorized subject declares `supersedes → S`; *annulled* iff its
   resolution standing says so; *deprecated* only through a
   `deprecates` relation on a successor; otherwise *current*. A manifest
   that writes `currency = "superseded"` or `"annulled"` is refused
   (`relations.currency-authored`): those are facts about a successor, and
   the predecessor's bytes are under a signature. `deprecated` as a bare
   field is tolerated for legacy manifests and reported.
3. **An atom is the complete answer to one question about one subject,
   under one jurisdiction.** Roles are a closed set per profile (§16);
   subjects are open. One atom per (subject, role) unless the profile
   marks the role repeatable, and then explicitly ordered (§16.2). No empty
   atoms: a role with nothing to say is omitted (§16.1). Atoms reference
   other subjects by identity, never by copying their text. The atom's
   `jurisdiction` is the only statement of how it may change: `authored`
   while unsigned, `bound` after, `generated` by `compile` alone.
4. **Only authored atoms are edited; their relation to a projection is
   their role.** The profile maps each role to the sections of the two
   projections that render it. An atom therefore carries its relation to
   the projections by construction — by naming a role — and a role no
   projection renders is refused (`atom.role-unprojected`). An explicit
   per-atom `projections:` list was rejected: it would be a second place
   to keep the same fact.
5. **Presets make authored atoms typed.** `war new <title> --preset
   <name>` writes each atom from a preset under
   `crates/openwarrant-cli/templates/presets/<name>/`: the headings the
   projection will render, each with the question it answers, and nothing
   to delete. `war check` reports a heading left unanswered
   (`atom.preset-unanswered`). Presets are the tool's; the answers are the
   author's; the projection is the reader's.

## Why not the alternatives

- **Keep four overviews and add a fifth "current" one.** The fifth would be
  the one nobody re-checks. Two projections with disjoint jobs — what is,
  and what was — are the minimum that answers both questions; the four
  become inputs.
- **A hand-written CURRENT.md.** A document that claims currency by
  assertion is the thing `roadmap.status-claim` already refuses for
  "resolved". A claim of currency must be computed or it is a date.
- **A `currency` field, set by the tool on supersession.** It is the
  predecessor's manifest — the tool would be editing a signed record. The
  relation lives on the successor, which is the record that is *being*
  signed.
- **Free-form atoms with a lint.** §8 asks for typed authored atoms. A
  preset is the type; a lint on prose is a guess.

## Consequences

- OW-WAR-0073's manifest mark is reverted; its revision-1 signature stands;
  `relations.currency` reads the derivation and passes. The basis bullet
  in OW-WAR-0112's `20-basis.md:47` that claimed the mark was digest-neutral
  was wrong; that atom is bound, so this ADR records the correction rather
  than an edit.
- `CORPUS_STATUS.md`, `WARRANT_OVERVIEW.md` and `ADR_OVERVIEW.md` remain
  as machine-adjacent views (the app and `war status` read the JSON forms);
  `README.md` and `QUICKSTART.md` point a reader at `CURRENT.md` first.
- `war next` and the app's Help pane hand a human a signing command only
  after the dry run has judged it.
- New Warrants are made from presets; the `TODO` skeleton is retired. The
  first preset set is small on purpose: `feature`, `fix`, `decision`.

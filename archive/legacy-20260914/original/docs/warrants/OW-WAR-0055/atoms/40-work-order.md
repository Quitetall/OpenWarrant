---
schema: oh.war/atom/v1
warrant_uuid: 01a06011-b342-78b3-8ba5-ed5c5cd9ba09
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `RoadmapRef::parse` beside `RequirementRef::parse`: grammar
   `roadmap://<PREFIX>-PHASE-<N>/<slug>`, N in 0..=10 unpadded, slug
   `[a-z0-9-]+`, slug optional for a phase-level ref. `war check` rejects bad
   grammar as `roadmap.malformed`.
2. `Contribution` validated on load: `Implements.contribution` outside §34.2's
   five values is `traceability.contribution` (error); absent is
   `traceability.contribution-unstated` (warn — §34.2 is SHOULD).
3. `traceability::derive_all` wired: `Implements.warrant_resolved` is true
   **only from a §56.2 record**, never from "would satisfy", so §34.3's
   `satisfied` keeps its meaning. A separate forward-looking `would_satisfy`
   count sits beside it. All 57 IDs seeded from §106 so `unaddressed` is listed
   by id with the requirement's text.
4. `CorpusStatus` in `openwarrant-core` (no I/O): releases, objectives (§98
   phases ascending, plus a synthetic `unassigned` listed last), warrants
   (including non-validating ones, marked), requirements, `next_actionable`,
   `provenance`. Every count is a ladder. Per-Warrant: the thirteen checks,
   §38.6 beside them and never folded in, blocking unknowns, milestone DAG state.
5. `assess()` extracted from `resolve::run` so the projection and
   `war resolve` compute from one function.
6. Bare `war status` (Markdown) and `war status --json` (JCS canonical bytes,
   `DigestDomain::CorpusStatus`). `war status <alias>` delegates to the
   per-Warrant `status` view per §72.5.
7. `CORPUS_STATUS.md` and `CORPUS_STATUS.json` under `docs/warrants/generated/`,
   written on full-corpus `war compile`, drift-checked by `war check --generated`.

## Frozen Surfaces

The §34.3 ladder names. `resolve::evaluate` — untouched. The thirteen
`ResolutionChecks` field names. The rule that no ratio is computed anywhere in
the projection.

## Premade Instructions

- The roadmap's "resolved" column is not a record. Do not read it.
- `next_actionable` is never empty-and-silent: if no stage is unblocked, emit
  the lowest unachieved Objective's blockers instead.
- Phases 9 and 10 report `exit_criterion: null` and `achieved: not_derivable`.
  Do not invent a criterion.
- Regenerate `deliverables.toml` as the LAST step. The record pins bytes, and
  this Warrant edits files other Warrants declare.

## Autonomy and Escalation

Tier T2. Escalate if the projection's JSON cannot be made byte-deterministic
across two runs — that would mean an unsorted collection or a timestamp, and
either is a design defect rather than something to work around.

## Rollback

Revert. The corpus returns to a flat overview in which every row reads
`draft`, which is the honest state it was in.

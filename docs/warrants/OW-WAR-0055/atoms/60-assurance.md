---
schema: oh.war/atom/v1
warrant_uuid: 01a06011-b342-78b3-8ba5-ed5c5cd9ba09
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a malformed roadmap reference is refused
- **scope:** §105 URI form, this repository's grammar.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant with `roadmap://OW-PHASE-11/x` and one with an
  uppercase slug are each rejected as `roadmap.malformed`.

### OBL-002 — a contribution outside §34.2's five is refused
- **scope:** §34.2.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant with `contribution = "mostly"` is rejected as
  `traceability.contribution` naming the five; an absent contribution warns
  and does not block.

### OBL-003 — requirement status is derived, and `satisfied` needs a resolution
- **scope:** §34.3.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** with zero resolution records on disk, every referenced
  requirement reports `claimed` or `in_progress`, none `satisfied`; the six
  unreferenced ones report `unaddressed` by id. A unit test proves a `complete`
  claim with `warrant_resolved = false` cannot reach `satisfied`.

### OBL-004 — the projection is byte-deterministic
- **scope:** §17.5, §65.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** two consecutive `war status --json` runs are byte-identical;
  `CORPUS_STATUS.json` committed and recompiled differs by nothing.

### OBL-005 — a hand-edited projection is caught
- **scope:** RQ-075.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant editing one byte of `CORPUS_STATUS.md` and one of
  `CORPUS_STATUS.json` is each rejected as `corpus-status.drift`.

### OBL-006 — an agent can find what to do next from one document
- **scope:** this repository's corpus.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war status --json` yields a non-empty `next_actionable` naming
  a stage in an unresolved Warrant in the lowest unachieved Objective, with a
  one-sentence `why`; a Warrant with no `[[roadmap]]` appears under
  `unassigned` and never in `next_actionable`.

## Gate Adequacy

Required at `controlled`.

**Adversarial question:** can the projection be made to print a number that
reads as progress but is not? The attacks that would do it: a ratio anywhere in
the renderer; `would_satisfy` folded into `satisfied`; the roadmap's
"resolved" column read as a record; non-validating Warrants omitted so the
denominator shrinks; `next_actionable` returning empty and silent.

**Executed attacks:** eight plants in `conformance/plant.sh`, each rejected by
its intended control on the first full run (109 passed, 0 failed):

- `roadmap://OW-PHASE-11/rationale` → `roadmap.malformed` (0..=10)
- `roadmap://OW-PHASE-1/Rationale` → `roadmap.malformed` ([a-z0-9-])
- `contribution = "mostly"` → `traceability.contribution` naming the five
- one byte of `CORPUS_STATUS.md` → `corpus-status.drift`
- `"satisfied":0` edited to `57` in `CORPUS_STATUS.json` → `corpus-status.drift`
- positive: `war status` names a `STAGE-` under "Next actionable"
- positive: `OW-WAR-0050`, which declares no `[[roadmap]]`, is listed under `unassigned`
- positive: two `war status --json` runs are byte-identical

One counterexample was found in the battery itself, not the projection: the
contribution plant first targeted OW-WAR-0014, which declares no
`[[implements]]`, so its sed matched nothing. The no-op guard refused to score
the unmutated corpus and exited 9. The plant was retargeted at OW-WAR-0009.

The ratio attack — "is there a division anywhere in the renderer?" — is held by
inspection of `corpus_status.rs`, which computes none, and by the module
doc-comment that says so; it has no plant because a plant can only show a
ratio is absent from one output, not from the code.

- **outcome:** counterexample_found, gate_added

## Residual Risk

- The roadmap grammar is a convention. If the SAS later binds
  `roadmap://<item-id>` to milestone ids, every slug here becomes a reference to
  nothing, loudly.
- "Objective achieved" rests on the `exit` slug convention. A phase whose exit
  Warrant is misnamed reports `not_derivable` rather than blocked, which is
  honest but easy to read past.
- The headline reads `0 satisfied` for as long as nothing is resolved. That is
  correct and will look like the tool is broken to a reader expecting a
  percentage. The document says why in one line; whether anyone reads the line
  is not something this Warrant can secure.

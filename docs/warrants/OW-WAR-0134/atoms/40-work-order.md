---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6016-7f51-b81b-3e8ccc3fdc35
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/acceptance.rs` (new).
   - For one resolved Warrant: the accepted tree (locator commit, with
     `paths_dirty` read from the pinned digests), the candidate tree, and
     the changed paths between them.
   - Each changed path is in-scope or out-of-scope under OW-WAR-0133's
     subject rule.
   - `UNKNOWN` when there is no locator, or git cannot read the commit.
2. `crates/openwarrant-cli/src/pins.rs` and `crates/openwarrant-cli/src/lib.rs`.
   - `war pins --candidate <rev>` (default `HEAD`) adds, per resolved
     Warrant: `unchanged`, `moved` with the in-scope paths, or `UNKNOWN`
     with the reason.
   - `acceptance.candidate-moved` is a diagnostic with that rule name, in
     `--json` too.
   - Exit code: non-zero on any in-scope move, under Q-001 (a) or (b).
3. `.github/workflows/ci.yml`: on `pull_request`, run
   `war pins --candidate HEAD --resolved-only` against the merge commit.
4. `conformance/plants.d/56-acceptance-validity.sh` (new): the plants in
   Assurance, on a scratch git corpus.
5. `docs/SIGNING.md`: a section "After you sign: when the candidate
   changes before merge", with what the finding means, what it does not
   mean (the resolution is not wrong), and what Q-001 requires next.
6. Under Q-001 (b) only: the re-verification record beside the resolution,
   through `war verify --run` on the new candidate. Its shape is the
   existing `oh.war/verification-response/v1`; no new schema.

## Frozen Surfaces

- `oh.war/resolution/v1`, including `locator`. Nothing here edits a
  resolution.
- `war check`: it stays git-free.
- `deliverable.digest-drift` and `resolution.stale` keep their meaning.

## Autonomy and Escalation

Tier T2. Stop and escalate on:
- any change that would write to `resolution.toml` or a dispute record;
- Q-001 (a): a new act kind is a SAS change and a human decision. It is
  not built here without one;
- any CI change beyond the one step.

## Rollback

Remove the CI step and the `--candidate` flag. No record was written, so
nothing is left to undo.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6016-7f51-b81b-3e8ccc3fdc35
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- RQ-059 and §56.2: a resolution binds the exact contract and assurance
  snapshot; `locator` names the commit whose tree held the delivered bytes
  at ingest.
- RQ-036 and §37.5: a delivered artifact changes only under a later
  authorized WAR that declares it, or through a recorded correction.
- §24.6: `standing` records whether reliance is still permitted; the
  outcome records what was concluded at the time.
- Product spec (`docs/design/openwarrant-product-spec.md`): Q11
  (acceptance may merge a verified change), Q23 (re-verify after the
  target branch changes), Q49 (mark before merge), and "Engineering
  contracts still to specify".
- Code read for this draft:
  - `crates/openwarrant-cli/src/resolution_cmd.rs`: `Locator`, `locate`,
    `check` (`resolution.stale`);
  - `crates/openwarrant-cli/src/pins.rs`: locator verification by
    `git show <sha>:<path>`, and its `UNKNOWN` cases;
  - `crates/openwarrant-cli/src/check.rs`: calls no git. `war check` stays
    deterministic and offline, so this finding is not added to it.
- OW-WAR-0133: the subject rule for evidence reuse (its Q-001).
- OW-ADR-0021: declaring an existing file makes this Warrant its owner on
  authorization. `pins.rs` and `lib.rs` are pinned by OW-WAR-0112 and
  others (authorized), `.github/workflows/ci.yml` by OW-WAR-0001.

## Assumptions

- A-001: the locator's `commit_sha` plus `paths_dirty` is enough to name
  the accepted tree for every resolution that has a locator. When
  `worktree_clean` is false, the dirty paths' accepted bytes are the
  pinned digests, not the bytes at `commit_sha`. Confidence: high; this is
  what the locator was built to say.
- A-002: `git diff --name-only <locator> <candidate>` is an honest list of
  what changed. Renames count as two paths. Confidence: high.
- A-003: GitHub's `pull_request` event checks out the merge commit, so the
  CI step sees the candidate that would land. Confidence: medium; a
  repository that merges by rebase sees a different final tree, and the
  check runs again on the pushed commit for that case.

## Unknowns

- **Blocking unknown — Q-001: what an in-scope change requires before
  merge.** Options:
  - (a) Merge is refused until the new candidate is re-verified and a
    human accepts it again. Today a second resolution is refused ("already
    carries a resolution"), so this needs a re-acceptance record: a new
    act, and a SAS revision.
  - (b) Merge is refused until the affected gates re-run and the
    independent verifier re-establishes every obligation on the new
    candidate. The human's acceptance carries forward, with the
    re-verification recorded beside it. No new human act.
  - (c) The finding is advisory: CI reports, a human decides, nothing is
    recorded.

  Recommendation: (b) for out-of-deliverable changes, (a) when a pinned
  deliverable itself had to change (which is already a correction today).
  (c) leaves Q23 unenforced.
- **Blocking unknown — OW-WAR-0133 Q-001.** The scope rule is shared. If
  it is not answered first, this Warrant can only report "changed" versus
  "unchanged" for the whole tree.
- U-003 (non-blocking): resolutions with no locator (recorded before
  OW-ADR-0021). They report `UNKNOWN` for every candidate. Backfilling a
  locator would be a guess, so this Warrant does not.

## Residual risks

- R-001: a squash merge makes every locator unreachable from main once the
  branch is deleted and collected. The finding is then `UNKNOWN` for good.
  Keeping Warrant branches, or recording the tree sha as well as the
  commit, would fix it; both change what a resolution records, which is
  frozen here.
- R-002: under Q-001 (b), the human accepted a tree they did not see. The
  re-verification is independent; it is still not the human's review.

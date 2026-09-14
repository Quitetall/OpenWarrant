# RC.2 implementation roadmap: planning validation

Observed 2026-09-14 in the isolated planning checkout on
`docs/rc2-implementation-warrants`, based on
`f22ef2f7282e8b5c72f2c4323b300f3b5e16102c`.

## Scope and results

- Created OW-WAR-0074 through OW-WAR-0091 with the repository-built `war new`.
  The tool allocated their local aliases and UUIDs and wrote draft-created
  journal events. No enterprise IDs, authorization, verification dispositions,
  SAS acceptance or resolutions were written.
- The planning index has 18 nodes, no missing predecessor or cycle, and assigns
  all F01–F11 and T01–T56 to implementation Warrants. The Phase 1 exit rechecks
  that complete inventory; the integrated release gate covers it again.
- `war check --json` after authored-record fixes: 732 pass, 88 warnings,
  zero errors. The initial milestone JSON serialization was valid general YAML
  but outside the repository's deliberately restricted YAML grammar. Authored
  atoms and matching proposal bodies were converted to the supported form;
  no checker was weakened.
- `war compile --json` regenerated all views through the tool. This installed
  command printed plain progress lines despite `--json`; no JSON envelope is
  claimed for that output.
- `war check --generated --json`: 846 pass, 88 warnings, zero errors. This is
  structural and projection evidence, not a full gate or approval.
- `war frontier <alias> --json` succeeded for all 18 drafts. An internal OPEN
  stage does not establish external prerequisite readiness or authorization.
- All 18 retained v2 proposals passed parse, authority/risk, schema and reference
  validation with zero failures. `war plan --proposal ... --json` returned
  process exit 2 and `plan.not-applicable`: semantic review/application steps
  were not claimed. The report's embedded exit_code was 0; the actual process
  exit and refusal are recorded here. These are reviewable synthesis records,
  not evidence that a human reviewed or applied an agent proposal. The authored
  drafts were created separately through the user-requested `war new` path.
- The retained RC.2 example audit passed: three Markdown sources, ten selected
  required units, six package files, exact text and digests intact. No production
  parser/compiler, condition conformance or workflow execution is established.

Before the later consolidation, the candidate source-set manifest was
byte-identical to the earlier local candidate: SHA-256
`003a1b407052ec6d833af11b9d37d1340448b4093659c052e3f28e9c2039c5c9`.
The separate [candidate validation record](../sas/drafts/1.0.0-rc.2/validation.md)
preserves the earlier worktree's observations and limitations.

## Remote facts at inspection time

- `origin/feat/battery-split` matched the local base commit via `git ls-remote`.
  RC.2 and its supporting design documents were untracked before this task;
  they were not committed or pushed then.
- Remote `v1.0.0` already targets
  `5ab3636597461e28712d777fed847150db6af8c3`.
- `gh release list` listed only `v0.1.0`, marked prerelease.
- HTTPS sparse-index requests to crates.io for `openwarrant-core`,
  `openwarrant-agent`, `openwarrant-compiler` and `openwarrant-cli` each returned
  HTTP 404. This is not a future availability guarantee.

The publication Warrant records the release-ref collision as a blocking owner
decision. No tag, release, registry entry or default branch was changed during
planning. External review is required after each commit; review output and final
remote-branch verification are reported separately from this pre-commit record.

## Boundaries

The root worktree's existing AGENTS.md and pinned README edits were not copied
into the planning branch. Their pending status is not erased by the clean
structural result in this isolated checkout. Existing SAS, ADR and Warrant
source records remain unchanged; corpus overview files change only through
compilation to reflect the additional drafts.

No aggregate `cargo xtask gate`, production RC.2 acceptance suite, platform
qualification, independent Warrant verification or Stable release ran for this
planning change. Those are named obligations in the new Warrants, not inferred
from document validation.

## Consolidation update

The owner requested consolidation from rc2-consolidated-base.md and confirmed
normative definitions with advisory usage notes, plus preserved signed legacy
history with successors when needed. SAS, format explanations, build cases,
decision map, adoption plan and every new Warrant basis now reflect those choices.
Ignored external_dependency fields were removed from the new rationale records;
manual prerequisite holds and their lack of automatic execution fencing are
explicit. No historical authorization or ADR source was edited.

The earlier SAS-preservation commit 2bca43c received LAMU review_commit
PASS WITH NITS through a fresh MCP process with automatic model selection.
The attached connection had returned Transport closed. Findings were checked:
catalog.required was already present; the committed source-set had 23 entries,
not an empty object; cross-document dependencies were already supported; the
audit compared the root preimage payload to the actual manifest before hashing.
Those alleged defects were not patched. The genuine ambiguous reason wording
now separates catalog diagnostic strings from structured inclusion reasons.
Empty access allowlists and cross-document dependencies are made explicit.
The LF-only fixed-example audit still does not claim production CRLF coverage.

New candidate manifest SHA-256: `201b707e9657a3c6674ea113a018a34a54b5ab425f37c417c8968af9f4d2cf31`.
This digest replaces the earlier candidate only in unsigned draft planning bases;
it is not an acceptance or a modification of an approved contract.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ff5-77b0-a389-10e0350f4b79
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a receipt names the source it ran over
- **scope:** receipts minted by `war evidence record` on a scratch corpus in
  a git repository. No claim about receipts minted by other tools.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a receipt from a clean tree names the contract, `tree:<sha>` equal to
    `git rev-parse HEAD^{tree}`, and `deliverables:sha256:<digest>`;
  - a receipt from a dirty tree also names `worktree:dirty`;
  - the receipt's seal recomputes in both cases.

### OBL-002 — a moved source makes the run a record, not evidence
- **scope:** `war check` and `war resolve --dry-run` requirement 5 on a
  scratch corpus, under the rule Q-001 selects.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - record a passing run; `war resolve --dry-run` counts it;
  - change one byte of a declared deliverable and commit; the same run is
    refused by `evidence.stale-binding`, naming the moved subject;
  - control: a commit that touches nothing the selected rule reads leaves
    the run admissible. Without this plant, a rule that refuses every
    receipt would satisfy the obligation.

### OBL-003 — a receipt that does not name its source is UNKNOWN, and history is untouched
- **scope:** receipts carrying only the contract subject; resolutions
  already recorded in this repository.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - a planted contract-only receipt on an unresolved scratch Warrant is
    `evidence.reuse-unknown`: not admissible, and not a gate failure;
  - on this repository, `war check` reports no new error for any resolved
    Warrant, and every `resolution.toml` is byte-identical before and
    after.

### OBL-004 — the context manifest does not claim an unchecked conflict list
- **scope:** Dispatches compiled by `war dispatch` for a scratch Warrant's
  stages.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** the emitted manifest's conflict field reads as Q-002
  selects: `unchecked`, or a list a check produced. The plant fails on
  `conflicts: []` with no statement that a check ran.

### OBL-005 — a required item is never omitted, and a detected conflict refuses the Dispatch
- **scope:** the omission rule in `context.rs` as reached through
  `war dispatch`; the conflict kind Q-002 selects, if any.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a stage whose budget cannot hold a required atom is refused by name
    (`RequiredItemOmitted`), not dispatched without it;
  - under Q-002 (c): one source path planted at two digests refuses the
    Dispatch and names both digests. Under Q-002 (a), this bullet is not
    claimed.

## Gate Adequacy

**Adversarial question:** could an artifact pass every declared gate while
a receipt recorded before a source change still counts as evidence about
the changed source?

Counterexamples the author considered while drafting (not executed):

1. A rule that refuses every receipt passes OBL-002's refusal half. Closed
   by OBL-002's control: an unrelated commit must leave the run admissible.
2. Under Q-001 (b), a gate that reads a file outside the deliverable set.
   Not closed: named as R-001, visible in the receipt's tree subject.
3. A receipt hand-edited to name the new tree. Closed by the existing seal:
   an edited receipt does not recompute (`evidence.rs`), and OBL-001
   checks the seal.

The §39.2 outcome and the executed attacks are left to the blind review
this level requires. The author does not record them.

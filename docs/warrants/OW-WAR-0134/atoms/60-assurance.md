---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6016-7f51-b81b-3e8ccc3fdc35
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — an in-scope change after acceptance is named
- **scope:** `war pins --candidate` on a scratch git corpus with one
  resolved Warrant whose locator is clean. No claim about forges other than
  a local git repository.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - after one commit changing an in-scope file that no deliverable pins,
    the output carries `acceptance.candidate-moved` naming that path, and
    exits non-zero;
  - `resolution.toml` is byte-identical before and after.

### OBL-002 — an unrelated change leaves acceptance standing
- **scope:** the same scratch corpus.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** a commit that changes only an out-of-scope file reports
  `unchanged` for the Warrant, and exits zero. This is the control: without
  it, a check that reports every Warrant as moved satisfies OBL-001.

### OBL-003 — a locator history cannot read is UNKNOWN
- **scope:** a scratch resolution with no locator, and one whose
  `commit_sha` names a commit not in the repository.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** both report `UNKNOWN` with the reason (no locator;
  commit not readable). Neither reports `unchanged`, and neither is an
  error about the resolution.

### OBL-004 — CI sees the merge candidate
- **scope:** `.github/workflows/ci.yml` as committed. No claim about a CI
  run on GitHub's hosts; that is observed after merge, not here.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - the workflow runs `war pins --candidate` on `pull_request`, and a
    non-zero exit fails the job;
  - `docs/SIGNING.md` states what the finding means and what Q-001
    requires next, in the words Q-001's answer uses.

### OBL-005 — the consequence Q-001 selects is enforced
- **scope:** the scratch corpus from OBL-001, after the in-scope move.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - under (b): the finding clears only after a verification of the new
    candidate is ingested, and clears for that candidate only; a second
    in-scope commit raises it again;
  - under (a): the finding clears only after the re-acceptance record the
    SAS revision defines;
  - under (c): this obligation is withdrawn by amendment, not left
    unsatisfied.

## Gate Adequacy

**Adversarial question:** could an artifact pass every declared gate while
an accepted candidate changes in scope before merge and nothing says so?

Counterexamples the author considered while drafting (not executed):

1. A check that always says "moved" passes OBL-001. Closed by OBL-002's
   control.
2. A check that says "unchanged" when git fails. Closed by OBL-003.
3. A change that lands only in the merge commit's conflict resolution.
   Covered only if CI checks the merge commit (A-003); a rebase-merge
   repository is covered by the run on the pushed commit, not before it.

The §39.2 outcome and the executed attacks are left to the blind review
this level requires. The author does not record them.

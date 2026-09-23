---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ef7-7862-8205-72d16ebcaf9f
role: assurance
jurisdiction: authored
order: 60
classification: internal
---


# Assurance

## Acceptance Obligations

### OBL-001 — a revision the parent never had is refused
- **scope:** `[[parents]]` entries whose parent is in this repository,
  exercised by `58-parent-revision.sh` on OW-WAR-0002. No claim about
  cross-repository parents.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** OW-WAR-0002 citing revision 7 makes `war check` report
  `relations.parent-revision` naming revision 7 and the parent's latest
  revision (2), and exit non-zero.

### OBL-002 — a digest of another revision is named as that revision
- **scope:** the same plant corpus, under the severity the owner chose
  (U-001).
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - OW-WAR-0002 citing revision 2 at `ab7e2df7…` produces
    `relations.parent-revision` whose message names revision 1;
  - a digest of all zeros produces `relations.parent-digest` as an error;
  - the plant fails if either finding is absent.

### OBL-003 — an exact older citation passes, and the move is reported
- **scope:** a child citing an older revision of a parent in this
  repository, with full Git history.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** OW-WAR-0002 citing revision 1 at `ab7e2df7…` yields no
  error from either parent rule, and one `relations.parent-moved` warning
  naming revision 2. Citing revision 2 at `f29c7d95…` yields neither.

### OBL-004 — what history cannot answer is UNKNOWN, and an unauthorized parent edit is still caught
- **scope:** a `git clone --depth 1` copy of the repository; the existing
  `00-corpus.sh` plant.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - in the shallow copy, the revision 1 citation is reported UNKNOWN
    under `relations.parent-revision`, and not PASS or ERROR;
  - "stale parent contract digest" in `00-corpus.sh` still fails with
    `relations.parent-digest`, unmodified.

### OBL-005 — war new --parent writes the exact citation, and refuses what it cannot cite
- **scope:** `war new --parent` on this repository, in a scratch copy.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - `war new "x" --parent OW-WAR-0001` writes `[[parents]]` with the
    parent's `war://` uuid, `contract_revision = 2` and
    `contract_digest = "sha256:f29c7d95…"` in full, and the new Warrant
    passes `relations.parent-revision`;
  - `--parent OW-WAR-9999` and `--parent` naming a draft Warrant each
    exit non-zero and create no directory.

### OBL-006 — the corpus result is the one the owner chose
- **scope:** this repository's corpus at the commit the battery runs on.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war check` reports each of OW-WAR-0002 to 0005 once
  under `relations.parent-revision`, naming revision 2, at the severity
  U-001's answer set, and reports no parent finding for any other
  Warrant.

## Gate Adequacy

Required at `basic`. The load-bearing plants are OBL-002's: a check that
accepts a revision number and a digest that name different revisions is
the defect the corpus already carries. OBL-003 shows the fix does not buy
that by forcing every child to follow its parent's latest revision.

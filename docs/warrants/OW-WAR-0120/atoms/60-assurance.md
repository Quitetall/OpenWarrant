---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5eaf-7041-b99b-eaed6ac66f86
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the generator builds real resolved Warrants, and only in its scratch directory
- **scope:** `tools/scale/synth-corpus.sh --n 12 --resolved 6` in a
  temporary directory, run by `52-retention.sh`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - `war check` on the result exits 0;
  - `war status --json` counts 6 Warrants resolved and 6 not;
  - the generator refuses a non-empty `--out` and writes nothing;
  - `git status --porcelain` of this repository is identical before and
    after.

### OBL-002 — the budget gate refuses what is over budget
- **scope:** `tools/scale/budget.sh` on the 12-Warrant corpus.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - with `tools/scale/budget.toml` it exits 0;
  - with every limit set to 1 ms it exits 1 and names every command in the
    budget.

### OBL-003 — a command that fails is unknown, never within budget
- **scope:** the same, with one manifest in the corpus made unparseable.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** each command that exits non-zero has an `unknown` row, no
  time counted as within budget, and the script exits 1.

### OBL-004 — the 1,000-Warrant measurement is recorded as measured
- **scope:** `docs/scale/baseline-1000.json` and the corpus that
  produced it.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the record names a release build, its `war --version`, the commit, the
    CPU and OS;
  - it states 1,000 Warrants and a resolved count of at least 500, as
    `war status --json` counted them;
  - `docs/RETENTION.md`'s numbers match the record.

### OBL-005 — the budget holds at 1,000 Warrants
- **scope:** the commands in `tools/scale/budget.toml`, release build,
  on the machine named in `baseline-1000.json`. No claim for another
  machine.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** `baseline-1000.json` shows every row within its limit and
  none `unknown`. If any row is over, this obligation is refuted and the
  record says which.

## Gate Adequacy

Required at `basic`. The load-bearing obligations are OBL-002 and OBL-003: a
budget gate that cannot fail, or that counts a failed command as fast,
would answer the owner's question with a number nobody measured.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5e81-73d3-b28f-cedb19f93c32
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — every tool step is timed, and every human step is counted, never timed
- **scope:** `tools/friction/measure.sh` run by `50-friction.sh` on a
  scratch directory, Linux x86_64. No claim about any other OS.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the record lists every setup step and every routine act in
    40-work-order, each tool step with an exit code of 0 and a time in
    milliseconds;
  - each human step has `time: not_measured` and a non-empty list of what
    it asks for;
  - the plant fails if any human step carries a number or a zero.

### OBL-002 — a failed step is unknown, and the run fails
- **scope:** the script, with a wrapper `war` on PATH that exits 1 for
  `authorize` and passes every other command through.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the `authorize` step's time is `unknown`;
  - the setup total is `unknown`, not the sum of the other steps;
  - the script exits 1.

### OBL-003 — the measurement leaves nothing behind
- **scope:** one run of the script from the repository root.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - `git status --porcelain` is identical before and after;
  - `SSH_AUTH_SOCK` after the run equals its value before;
  - the ssh-agent the script started is no longer running;
  - no file under `~/.ssh` changed (the plant runs with `HOME` pointed at a
    temporary directory and checks it holds nothing afterwards).

### OBL-004 — the baseline is the script's own output, from a release build
- **scope:** `docs/friction/baseline-1.json` and `docs/FRICTION.md` at
  delivery.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the baseline parses as the script's record, names build profile
    `release`, a `war --version` and a commit;
  - re-running the script with the same binary produces a record with the
    same steps in the same order (times may differ);
  - every table row in `docs/FRICTION.md` matches the baseline's medians;
  - the doc names each human step with `not measured`, and says the setup
    target is neither met nor missed by this evidence.

## Gate Adequacy

Required at `basic`. The load-bearing obligations are OBL-001 and OBL-002: a
friction number that silently treats a human step as free, or a failed step
as fast, would report the target met when nobody measured it.

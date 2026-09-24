---
schema: oh.war/atom/v1
warrant_uuid: 01a0d31e-63a1-7092-a4d2-0937d817adcb
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the gate is askable, runs, and leaves the working tree untouched
- **scope:** `war gate --run --gate ops.conformance.plants@1.1.0` on a
  scratch corpus with a fake battery script; the corpus carries an
  uncommitted marker file and an uncommitted edit to a tracked file.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - the run is askable and completes with verdict pass, and its output
    names the tested commit, which equals `HEAD`;
  - the working tree (every file, the index, `HEAD`) is byte-identical
    before and after, including the uncommitted marker and edit;
  - no clone directory is left under the temp directory;
  - refusal: the fake battery exiting 1 makes the verdict fail; the
    uncommitted edit is not seen by the fake battery (it reads `HEAD`'s
    bytes).

### OBL-002 — a battery inside the battery refuses instead of recursing
- **scope:** `conformance/plant-isolated.sh` run with
  `OPENWARRANT_IN_BATTERY=1`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** exit 2 with a line naming the refusal, and no clone made.

### OBL-003 — the new version is registered and the old one is untouched
- **scope:** `war check` on this repository after delivery.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `gate.registered` PASS for `ops.conformance.plants@1.1.0`
  as `mutating false`; `@1.0.0`'s file is byte-identical to before; `war
  check` reports 0 errors.

## Gate Adequacy

Required at `basic`. OBL-001's working-tree comparison is the load-bearing
check: a gate that claimed `mutating: "false"` while touching the tree
would be exactly the misdeclaration §44.8 exists to prevent.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fbc-7de3-83cc-00bd20244dc6
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a cancelled war perform leaves no writer behind and no submission
- **scope:** `war perform` on Linux in the conformance battery, with
  `writes-until-killed.sh` as the performer, sent SIGINT and SIGTERM. macOS
  is claimed only if the plant also runs on a macOS runner; otherwise it is
  reported not established. No claim about a process that leaves its group.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - After SIGINT to `war`, the report is `perform.cancelled` and exit is
    non-zero. The marker file's line count is unchanged 1 s later, for
    both the performer and its child. No `submissions/*.json` exists, and
    the Dispatch file is kept.
  - The same for SIGTERM.
  - Negative control: the same fixture started in its own group by a
    plain shell (`setsid`), whose parent is then killed, keeps growing its
    marker. The plant's check reports that as an orphan, so it can see one.
    No switch in the shipped binary disables the handler.

### OBL-002 — a second writer is refused while the first may still write
- **scope:** `war perform <alias> <stage>` twice on the same stage, the
  first still running; and a planted `.writer` record.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - With the first performer running, the second call is refused
    `perform.writer-alive`, spawns nothing (a second marker file is
    absent), and compiles no Dispatch (no new `dispatch.compiled`).
  - A `.writer` naming a pid whose start time does not match is refused
    `perform.writer-unknown`.
  - A `.writer` naming a group that is gone lets the second call proceed,
    and the record is removed. The refusal is not blanket.

### OBL-003 — an OS without the group kill is refused, not run weaker
- **scope:** the `cfg` gate in `perform.rs`, as compiled for the release
  targets. The refusal branch is exercised by a unit test that forces the
  unsupported path.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - The unit test forcing the non-unix branch gets `perform.unsupported-os`,
    and no spawn is attempted (the fake spawner records zero calls).
  - On Linux, the plants in OBL-001 and OBL-002 run, which shows the
    supported branch is the one compiled.

### OBL-004 — the claude adapter passes the model's answer or nothing
- **scope:** `tools/performer/claude-performer.sh` with a fake `claude` on
  PATH that answers from fixtures. No claim about a real model's work.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - A fixture submission naming the right dispatch is printed byte for byte
    and ingested by `war perform`.
  - A fixture naming another dispatch, one requesting resolution, one that
    is not JSON, and a fake `claude` that exits 1 each give adapter exit
    non-zero with empty stdout, and `war perform` records no submission.
  - The recorded argv carries the tool list U-002 settled. The plant fails
    if the list changes.

## Gate Adequacy

Required at `basic`. The load-bearing obligation is OBL-001. A performer
that keeps writing after its command is gone is the unbounded authority
RQ-044 forbids, and the growing marker file is the observation that shows
it. OBL-001's negative control is what shows the plant can see an orphan
at all.

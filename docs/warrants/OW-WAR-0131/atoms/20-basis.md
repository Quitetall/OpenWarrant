---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fbc-7de3-83cc-00bd20244dc6
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- RQ-044: agent authority is explicit and bounded. A performer that
  outlives the command that started it is unbounded.
- §55.2: headless execution with no permitted approval path fails closed.
  An unsupported OS is refused, not run with a weaker bound.
- §55.7: budget exhaustion halts honestly. A cancelled run leaves no
  submission.
- §51.2: a performer cannot complete its own work. A cancelled or killed
  performer's partial answer is discarded, as `perform.rs` already does on
  timeout.
- Product spec, "Engineering contracts still to specify": supported
  agent/harness adapters on each OS, actual protection checks, exclusive
  writer handoff, cancellation guarantees. It also says "a replacement may
  write only after the previous writer can no longer write. Missed
  heartbeats alone do not prove that."
- CONTEXT.md: an agent/harness stop is interrupted execution, not
  completion.
- OW-WAR-0069 (authorized): delivered `war perform`. Its Q-005
  (performer configured once per repository) and Q-006 (one at a time) are
  unanswered, and this Warrant follows their recommendations.
- `.github/workflows/release.yml`: release targets are
  `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`.

## What exists (read 2026-09-23, branch `claude/hub`)

- `perform.rs`:
  - `hand_over` spawns with `process_group(0)` on unix and polls
    `try_wait`;
  - `kill_group` runs `kill -KILL -<pgid>`;
  - there is no signal handler and no writer record, and `run` does not
    read the frontier.
- `frontier.rs`: claimed = a `dispatch.compiled` event with no
  `submission.recorded`.
- `Cargo.toml` (cli): `rustix` with `process` on unix, so a group-liveness
  probe needs no new crate. `signal-hook 0.3.18` is already in `Cargo.lock`
  as a transitive dependency.
- `perform.rs` is declared by no Warrant's deliverables. OW-WAR-0132 also
  declares it (see R-002).

## Assumptions

- A-001: a process group that `kill(-pgid, 0)` reports as nonexistent
  (ESRCH) can no longer write. Confidence: high on Linux and macOS for
  processes that stayed in the group. Escapees are out of scope and named.
- A-002: a recorded pgid is not reused within the time a stale writer
  record sits on disk. Confidence: medium. The record also carries the
  leader's start time, and a mismatch is `perform.writer-unknown`, not
  "stopped".
- A-003: `claude -p` reads a prompt on stdin and can be told to answer
  with JSON only. Confidence: medium. The adapter validates its own output
  before printing, and a non-conforming answer exits non-zero with nothing
  on stdout.

## Unknowns

- U-001 (non-blocking; blocks STAGE-002): the signal handler.
  - A (recommended): add `signal-hook = "=0.3.18"` as a direct dependency.
    It is already locked through crossterm, so this adds no new code to
    the build.
  - B: tokio's `signal` feature. This widens a workspace dependency that
    OW-ADR-0014 confines to `mcp/`.
  - C: keep the performer in `war`'s process group, so the terminal's
    SIGINT reaches both. The deadline's group kill then kills `war` too,
    so this means redesigning the deadline.
- U-002 (non-blocking): which tools the claude adapter allows. The
  adapter's job is to write the work, so Edit and Bash are expected. The
  list is pinned in the plant's recorded argv so a change is visible.
  Owner may narrow it.

## Residual risks

- R-001: SIGKILL to `war` itself cannot be caught. The performer survives,
  and its `.writer` record stays behind. The next `war perform` sees a live
  group and refuses `perform.writer-alive`, which is the safe side.
- R-002: OW-WAR-0132 also changes `perform.rs`, to add admission checks.
  Under OW-ADR-0021 the later authorization governs. The owner should
  authorize this Warrant first and 0132 after it, or merge the two
  deliveries.

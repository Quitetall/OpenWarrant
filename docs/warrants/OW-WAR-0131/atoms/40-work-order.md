---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fbc-7de3-83cc-00bd20244dc6
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/perform.rs`:
   - **OS admission.** On a target without the unix group kill, refuse
     `perform.unsupported-os` before compiling or spawning.
   - **Writer record.** After spawn, write `dispatches/<id>.writer` (pid,
     pgid, leader start time, stage, dispatch id), and remove it after
     reaping.
   - **Handoff.** Before compiling, read every `.writer` for this stage.
     - Group alive: refuse `perform.writer-alive` naming pid and dispatch.
     - Probe inconclusive (EPERM, start-time mismatch): refuse
       `perform.writer-unknown`.
     - ESRCH: record the old writer as stopped, remove its record, and
       proceed.
   - **Cancellation.** SIGINT and SIGTERM set a flag the wait loop reads.
     The group is killed and reaped, and the answer is discarded. The
     report is ERROR `perform.cancelled`, and `war` exits non-zero with the
     Dispatch on record. `--all` stops after the current stage.
2. `crates/openwarrant-cli/Cargo.toml` and `Cargo.lock`: the signal handler
   dependency, per U-001 option A.
3. `tools/performer/claude-performer.sh`:
   - reads the Dispatch on stdin and runs `claude -p` with the Dispatch as
     the prompt and a fixed instruction to answer with one
     `oh.war/stage-submission/v1`;
   - validates the answer's `dispatch_id`, and that `requested_next_action`
     is not a resolution;
   - prints it, or exits non-zero with empty stdout;
   - never writes a submission `claude` did not produce;
   - `CLAUDE_PERFORMER_LOG` keeps the raw answer.
4. `conformance/fixtures/performer/writes-until-killed.sh`: appends a line
   to a marker file every 100 ms and starts a child that does the same.
5. `conformance/plants.d/60-perform-cancel.sh`, the plants the obligations
   name.
6. `docs/PERFORM.md`:
   - supported OSes;
   - what cancellation and handoff guarantee, and what they do not (an
     escaped process, SIGKILL to `war`, a remote writer);
   - how to configure the claude adapter;
   - that an interrupted run is not completion.

## Frozen Surfaces

- `oh.war/stage-dispatch/v1` and `oh.war/stage-submission/v1`.
- `war submit`'s refusals.
- `[perform]`'s existing keys, and `max_concurrent = 1`.
- The frontier's definition of claimed.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any path that lets a second writer start while the first's liveness is
  unknown;
- any widening of the adapter's allowed tools beyond U-002's recorded list;
- support for a third OS.

## Rollback

Revert `perform.rs`, the dependency, the adapter and the doc. Stale
`.writer` files are inert to the old code. Delete them by hand after
confirming that their groups are gone.

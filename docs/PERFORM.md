# `war perform`: what it guarantees, on which OS, and what it does not

`war perform <alias> <stage>` compiles an agent stage's Dispatch, hands it to
the performer named by `[perform] performer_argv` on stdin, and ingests the
Stage Submission it prints through the same refusals `war submit` applies
(OW-WAR-0069). This page states the bounds OW-WAR-0131 added: which OSes are
supported, what cancellation and writer handoff guarantee, and where each one
stops. Each claim names the plant or test that exercises it.

## Supported OSes

| OS | Supported | Why |
|---|---|---|
| Linux | yes | The performer leads its own process group; the deadline and a cancellation kill the whole group. |
| macOS | yes, with the same code | The same unix group kill. The cancellation plant runs on Linux; until it also runs on a macOS runner, macOS is claimed by construction, not by observation. |
| Anything else (Windows included) | **refused** | No group kill here, so a performer's children would outlive its deadline. `war perform` refuses with `perform.unsupported-os` before it compiles or spawns anything, rather than running under a weaker bound than it reports (§55.2). Compile the Dispatch with `war dispatch` and hand it over yourself. |

Exercised by: the unit test
`a_platform_without_the_group_kill_is_refused_before_anything_runs` in
`crates/openwarrant-cli/src/perform.rs` (a fake host with no group kill: the
refusal fires and no spawn is attempted), and `conformance/plants.d/60-perform-cancel.sh`,
whose runs on Linux show the supported branch is the one compiled.

## Cancellation

**Guaranteed.** When `war perform` gets SIGINT (Ctrl-C at the terminal) or
SIGTERM while a performer runs:

- the performer's whole process group gets SIGKILL and is reaped, so the
  performer and every child that stayed in its group stop writing;
- its answer is discarded, even when it had already finished. No
  `submissions/*.json` is written. A cancelled run leaves no submission (§55.7);
- the Dispatch stays on record under `dispatches/`, unanswered;
- the report is `ERROR perform.cancelled`, naming the signal, and `war` exits
  non-zero;
- `war perform --all` stops after the stage it interrupted. The later stages are
  not started, and the report says how many.

A signal that arrives after the Dispatch is compiled but before the performer is
spawned also cancels, and nothing is started. Before that point, and outside
`war perform`, SIGINT and SIGTERM keep their ordinary meaning.

**An interrupted run is not completion.** A cancelled stage is interrupted
execution. It is not a submission, a blocker, or a failure the performer
reported, and nothing about it counts toward resolving the Warrant.

Exercised by: `60-perform-cancel.sh`. For each of SIGINT and SIGTERM, the
fixture `conformance/fixtures/performer/writes-until-killed.sh` appends to a
marker file every 100 ms, and so does a child it starts. After the signal, the
report is `perform.cancelled` with a non-zero exit, the marker is unchanged one
second later for both the performer and the child, neither pid is alive, there
is no submission, and the Dispatch is kept. The negative control starts the
same fixture under `setsid` from a plain shell and kills that shell. The same
check reports it as an orphan, which shows the check can see one.

**Not covered:**

- **A process that leaves the group.** A performer or tool that calls `setsid`,
  double-forks into a daemon, or asks another service (systemd, launchd, a
  container runtime, a remote API) to run something is outside the group, and
  the group kill does not reach it. Nothing here contains a performer: no
  sandbox, no cgroup. This is why `[perform] max_concurrent` stays 1.
- **SIGKILL to `war` itself**, or `war` dying any other way it cannot catch.
  The performer survives. Its writer record stays on disk, and the next
  `war perform` on the stage refuses `perform.writer-alive` until its group is
  gone. That is the safe side, but somebody has to stop the orphan by hand.
- **A process stuck in uninterruptible sleep** takes the SIGKILL only once its
  I/O completes. The reap waits up to 5 s. After that the writer record is kept,
  and the next run refuses on it.

## One writer at a time (handoff)

**Guaranteed.** While a performer runs, `dispatches/<dispatch>.writer` records
its pid, its process group, the leader's start time, the stage, and the
Dispatch. The record is removed only once the group is shown gone. Before a new
`war perform` on the same stage compiles anything, it probes every such record:

| The probe finds | Result |
|---|---|
| A process still in the recorded group (`kill(-pgid, 0)` succeeds) and the leader's start time matches, or the leader is gone but its group is not | `ERROR perform.writer-alive`, naming the pid and the Dispatch. Nothing is compiled or spawned. |
| The probe cannot tell: `EPERM`, a live pid whose start time does not match (the pid may have been reused), or an unreadable record | `UNKNOWN perform.writer-unknown`. Nothing is compiled or spawned. Escalate: confirm by hand that the group is gone, then delete the record. |
| No process left in the group (`ESRCH`) | `PASS perform.writer-stopped`. The record is removed, and the new run proceeds. |

A record's age is never read as proof. Missed heartbeats do not show that a
writer has stopped, and neither does an old file.

If a performer exits normally but leaves a child running in its group, the
record is kept and the report adds `WARN perform.writer-lingering`. The next
run on the stage refuses until that child is gone.

Exercised by: `60-perform-cancel.sh`:

- With the first performer running, a second call is refused
  `perform.writer-alive`. Its performer never starts (its marker file is
  absent), and no new `dispatch.compiled` event is journalled.
- A planted record naming a live process with the wrong start time is refused
  `perform.writer-unknown`.
- A planted record naming a group that is gone is handed off: the record is
  removed and the performer starts.

The unit test `a_writer_is_alive_gone_or_unknown_and_never_guessed` covers
the probe's three answers.

**Not covered:**

- **Two `war perform` calls started at the same instant.** Each probes, finds no
  writer, and compiles. The record is written once the performer is spawned, so
  in the window between probe and spawn neither call can see the other. One
  operator running one command at a time does not hit this. A scheduler that
  starts several at once would.
- **A writer on another machine**, or in another checkout of the same
  repository. The record lives in this working tree and names a local process
  group. A remote writer is not probed, and not excluded.
- **Other stages.** Handoff is per stage. `max_concurrent = 1` is a policy of
  this command, not a lock across the repository.

## The claude adapter

`tools/performer/claude-performer.sh` turns a Dispatch into a `claude -p` run
and a Stage Submission. To use it, set it as the performer in
`openwarrant.toml`:

```toml
[perform]
performer_argv = ["tools/performer/claude-performer.sh"]
performer_timeout_secs = 1800
max_concurrent = 1
```

`claude` must be on `PATH` and signed in, and `python3` must be available.
`CLAUDE_PERFORMER_LOG=<file>` appends each raw answer and claude's stderr to
that file, whether or not the answer was accepted.

What it does:

- It runs `claude -p --output-format text --allowedTools Read,Glob,Grep,Edit,Write`,
  with a fixed instruction and then the Dispatch as the prompt on stdin.
- It prints claude's answer byte for byte, but only if the answer is one
  `oh.war/stage-submission/v1` JSON object whose `dispatch_id` is the
  Dispatch's own, and whose `requested_next_action` is one of §51.2's five
  (`continue`, `verify`, `block`, `amend`, `cancel`).
- Otherwise it exits non-zero with **empty stdout**, and `claude-performer:
  refused: <reason>` on stderr. The reason is one of `dispatch-mismatch`,
  `self-completion`, `not-json`, `not-a-submission`, `claude-failed`, or
  `not-a-dispatch`. `war perform` then reports `perform.failed`, and records
  nothing.
- It never writes a submission that `claude` did not produce.

What it grants: the five tools above, and nothing else. In `-p` mode a tool
outside the list has no approval path and is refused. **Bash is not on the
list**, because a shell can start a process outside the performer's group,
which is the one case cancellation does not cover. Widening the list is the
owner's decision. The plant pins the argv, so a change fails the battery until
the plant changes with it. The adapter adds no timeout, no concurrency, no
directories outside the repository, and no permission mode. The bound is
`war perform`'s: the adapter and `claude` are one process group, and a deadline
or cancellation kills both.

What it cannot promise: the quality of the work. `war perform` ingests the
submission, and an independent verifier judges it. The adapter shows only that
what came back is a legal answer to this Dispatch.

Exercised by: `60-perform-cancel.sh`, with a fake `claude` first on `PATH` that
answers from fixtures and records its argv. A real model is never run there.
The plant checks that:

- the right answer is printed byte for byte and ingested by `war perform`
  (`perform.answered`, `submission.recorded`);
- an answer naming another dispatch, one requesting resolution, one that is not
  JSON, and a `claude` that exits 1 are each refused with empty stdout, by the
  named reason, and `war perform` records no submission;
- the recorded argv matches the pinned tool list.

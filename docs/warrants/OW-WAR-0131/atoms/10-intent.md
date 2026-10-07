---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fbc-7de3-83cc-00bd20244dc6
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`war perform` (OW-WAR-0069) hands a Dispatch to `[perform] performer_argv`,
bounds it in wall time, and ingests the answer through `war submit`. Three
things the product spec asks of "agent/harness adapters on each OS, …
exclusive writer handoff, and cancellation guarantees" are not there.

- **Cancellation does not reach the performer.**
  - The performer runs in its own process group (`process_group(0)`), so
    the deadline's group kill reaches its children.
  - The same choice means a Ctrl-C at the terminal reaches `war`, not the
    performer. `war` has no signal handler.
  - `war` dies, and the performer and everything it started keep writing
    the tree, with nothing recording that they exist.
- **No writer handoff.**
  - A stage is "claimed" once a `dispatch.compiled` event names it
    (`frontier.rs`).
  - `war perform <alias> <stage>` does not look. It starts a second
    performer on a stage whose first may still be running.
  - Nothing records which process is the writer, so nothing can say
    whether the old writer has stopped.
- **No adapter, and no OS statement.**
  - The only performers are the fixtures under
    `conformance/fixtures/performer/`.
  - `perform.rs` notes that the group kill is unix-only, and that a
    Windows port "owes this function a job object".
  - On a platform without it, `war perform` runs anyway, with a weaker
    bound than it reports.

## Desired Outcome

- **Cancellation.** SIGINT or SIGTERM to `war perform` kills the
  performer's whole group and reaps it. The Dispatch stays on record, no
  submission is written, and the report says `perform.cancelled`. An
  agent/harness stop is interrupted execution, not completion (CONTEXT.md).
- **One writer, handed off only when the old one has provably stopped.**
  - While a performer runs, `dispatches/<id>.writer` records its pid,
    process group and start time.
  - A new `war perform` on the same stage refuses while that group is alive
    (`perform.writer-alive`).
  - If liveness cannot be established, it refuses as unknown
    (`perform.writer-unknown`).
  - It proceeds only when the group is shown gone. An old record's age
    alone is not proof.
- **Adapters per OS, stated.**
  - Linux and macOS (the two release targets) are supported with the
    group kill.
  - Any other platform is refused before spawning (`perform.unsupported-os`),
    not run with a weaker bound.
  - One real adapter, `tools/performer/claude-performer.sh`, turns a
    Dispatch into a `claude -p` run and a Stage Submission.
  - `docs/PERFORM.md` states what each guarantee does and does not cover.

## Non-goals

- A Windows port or job objects. Refusing is the deliverable.
- Containment: a sandbox, cgroups, or `max_concurrent` above 1.
  OW-WAR-0069 Q-006 stands.
- Katana (§48, OW-WAR-0026) or the reference web app's harness protocol
  (`apps/openwarrant-web`, OW-WAR-0094 and 0109). This is `war perform`'s
  own seam.
- Automatic replacement of an unknown writer. The product spec says escalate.
- A performer that escapes its group (`setsid`, a daemon). This is named
  in `docs/PERFORM.md` as uncontained, not claimed.

# Example 3 — a run Warrant: the conformance battery as a service stage

OW-WAR-0066, *Run: the conformance battery as a service stage*. Two
`service` stages, each naming a registered gate as its executor:

| stage | executor | wall time |
|---|---|---|
| STAGE-001 | `gate://ops.echo@1.0.0` (`true`) | 5 s |
| STAGE-002 | `gate://ops.conformance.plants@1.0.0` (`bash conformance/plant.sh`) | 1800 s |

## What a run leaves behind

```bash
war run OW-WAR-0066 STAGE-001
ls docs/warrants/OW-WAR-0066/{dispatches,gate-runs,submissions}
war journal OW-WAR-0066 | tail -2
```

- `dispatches/<dispatch-id>.json` — the Stage Dispatch that was compiled for
  the run: the stage's context (STAGE-001 narrows it to the work order's
  Premade Instructions), its token estimate and budget, its digest.
- `gate-runs/ops_echo_1_0_0.{run.toml,receipt.json,stdout.txt,stderr.txt}` —
  the gate run and its §44.6 receipt. The receipt's subject is
  `dispatch:<dispatch digest>`: the run is evidence about *what was
  dispatched*, bound to the contract that dispatch was compiled under.
- `submissions/<dispatch-id>.json` — the Stage Submission (§51). Its
  `requested_next_action` is `verify` when the gate passed and `block` — with
  a Blocker naming the gate, the bound and the verdict — when it failed or
  timed out. It is never anything else: a run cannot ask to be resolved.
- two journal events: `dispatch.compiled` (with the estimate) and
  `submission.recorded`.

The wall-clock bound is the smaller of the stage's `wall_time_seconds` and
the gate's own `timeout_secs`; a run that exceeds it is killed, its
submission requests `block`, and the report says `run.timeout`.

## Submissions from elsewhere

An agent or a BLUT job that performed a stage hands its submission to
`war submit OW-WAR-0066 <file>`. Two refusals stand between the file and the
record, and both leave nothing written: the submission must name a dispatch
this Warrant compiled (a `dispatch.compiled` journal event), and it may ask to
`continue`, be verified, `block`, `amend` or `cancel` — a request to be
resolved is refused by name (§51.2).

## What to copy

- `executor_kind: "service"` and `executor_ref: "gate://<key>"` on the stage,
  with a `wall_time_seconds` that means something;
- a gate definition whose `argv` is the run, registered under `docs/gates/`;
- obligations whose evidence is the receipt, not the log line.

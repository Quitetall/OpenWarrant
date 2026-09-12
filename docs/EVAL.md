# `war eval` — the agent loop, measured

`war eval run` answers one question per task: given a vague sentence, does
the loop around a drafter — draft, apply, check, dispatch, perform, evidence,
blind verification — leave a Warrant a human COULD resolve as satisfied?

It never authorizes, resolves or signs. Each task runs in a throwaway program
(`war init --program`, namespace `EV`) in a temp dir; every step is a child
`war --json`, so the harness measures what a user gets.

## Tasks

`evals/tasks/<id>/task.toml` (`oh.war/eval-task/v1`): the sentence verbatim,
`kind` (code | document | run), `expected_gate`, `max_tokens`, `max_wall_secs`,
and the `expected` rung. Beside it, what the fixture agent needs:
`proposal.json` (an `oh.war/draft-proposal/v2`, `{{NS}}` filled at draft
time), `deliver/` (copied into the scratch at perform time; `*.tmpl` files get
`{{ALIAS}}` and `{{DIGEST:<path>}}` filled) and `fixture/` (copied over the
scaffold before anything runs — gate definitions, mostly).

Twelve tasks ship: four per kind, their sentences drawn from Warrants this
repository actually resolved.

## The ladder

| rung | meaning |
|---|---|
| `would_satisfy` | §38.6 `would_resolve_satisfied` under the blind verifier, AND every requirement the loop controls holds |
| `partial` | the loop ran to the end; something it controls is unmet (a deliverable, a digest, a gate result, independence) |
| `over_budget` | request + Dispatch + bundle estimate exceeded `max_tokens` |
| `refused` | a named refusal stopped the loop (`plan.drafter-wrote-files`, `dispatch.over-budget`, `verify.inadmissible`, …) — the tool did its job |
| `errored` | a step failed for a reason that is not a refusal |

Six of the thirteen resolution requirements are answered from the authority
context — an authorization, roles, judgments, a resolver — and cannot hold in
a program no human has touched. The record lists them under
`requirements_unmet` every time and lists what the loop itself left undone
under `requirements_unmet_by_the_loop`, which is empty for `would_satisfy`.

`--reviewed` is passed by the harness: §74.4 steps 5–6 are a human review,
waived inside the scratch program and said so in every task's notes.

## Drafter and performer

The same argv is invoked twice per task:

- **draft**: the `oh.war/draft-request/v1` on stdin, a proposal on stdout,
  no file written (the tool refuses otherwise);
- **perform**: the argv plus a trailing `perform`, an `oh.war/eval-perform/v1`
  document on stdin (`task`, `kind`, `warrant`, `stage`, `repo_root`, the
  compiled `dispatch`), and the deliverable written into the scratch. Stdout
  is recorded, not parsed. For the run kind the tool performs (`war run`) and
  the performer only pins the receipt.

`evals/fixtures/fixture-agent.sh` is the free one: byte-deterministic, so the
baseline is comparable. A model-backed drafter is any command speaking the
same two modes.

## Results and the baseline

`war eval run` writes canonical JSON (`oh.war/eval-result/v1`) to
`evals/results/<sha>-<drafter>.json` (or `--out`), with timings in a sibling
`.timing.json` so the result itself is deterministic. Per task: the rung, the
expected rung, tokens (request, Dispatch, bundle, method), obligations
established/total, requirements unmet, refusals hit, every step's exit code
and verdict, and the §74.4 gauntlet as `plan.apply` reported it.

`war eval verify <result>` compares against `evals/baseline.json` one task at
a time: `eval.same`, `eval.improved`, `eval.regressed` (an ERROR), `eval.moved`,
`eval.new-task`, `eval.missing-task`. Never a percentage.

`.github/workflows/eval.yml` runs the fixture weekly and on demand, never per
PR. The conformance plants in `conformance/plants.d/87-eval.sh` prove the
rungs can be reached: a file-writing drafter is `refused`, a one-token budget
is `over_budget`, two fixture runs are byte-identical, a doctored result is
`eval.regressed`.

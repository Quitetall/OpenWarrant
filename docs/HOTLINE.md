# The hotline: how a question blocks, who can answer, and the limits on a performance

An agent asks and a human answers (§27.2, OW-WAR-0069). This page covers what
OW-WAR-0132 added around that exchange:

- what a blocking question does to its stage;
- what happens when nobody can answer it;
- the time, spend and attempt limits on `war perform`, and their defaults;
- what is not counted.

Each claim names the plant that exercises it. They are all in
`conformance/plants.d/52-hotline-defaults.sh`, on a scratch program with
fixture performers only.

## A blocking question stops its stage, and only that stage

`war ask <alias> <stage> "<question>" --blocking` records
`questions/Q-nnn.toml`. Until a human answers it:

- `war frontier` shows the stage `blocked`, and its `waiting_on` lists the
  question id, after any milestone the stage also waits on. This is a new
  value in an existing field; the `oh.war/frontier/v1` schema is unchanged.
- `war perform <alias> <stage>` refuses with `perform.question-open`, naming
  the question and the command that answers it. The refusal comes before
  anything is compiled or spawned.
- `war perform --all` skips the stage, because it is not open, and performs
  every other open agent stage.

Only a blocking question that is still unanswered blocks. A question asked
without `--blocking` stops nothing: it asks for a confirmation the performer
could go on without. Once `war answer … --as <human>` records an answer, the
stage is open again and `war perform` runs it.

A stage that already has a recorded submission shows `done` even with a
blocking question open on it. `done` still wins, as it always has, because a
recorded submission is history. `war perform` on that stage is still refused
`perform.question-open`.

If a question record cannot be read, `war perform` refuses with UNKNOWN
`perform.question-unknown`, and `war frontier` reports UNKNOWN
`frontier.question-unreadable`. A record nobody can read is not treated as a
question that does not exist.

Plants: "a blocking question blocks", "perform on a question-blocked stage"
(the marker performer leaves no marker and no Dispatch is written),
"--all performs the other stage", "a non-blocking question", and "an answered
question unblocks".

## Asking mid-run (OW-WAR-0069 Q-002)

A performer may ask while it runs, and only its own stage waits. This page
assumes the answer Q-002 recommends, which is still unanswered.

`war` cannot pause a running process to wait for an answer, and it does not
try. A performer whose blocking question is still unanswered when its wall
time runs out should end with a Stage Submission that asks to block
(`requested_next_action: "block"`) and cites the question id in its
`blockers`. `war submit` already admits that. It is the performer's own act:
nothing here writes it on the performer's behalf. If the performer does not do
this, the run ends as a timeout, and the open question keeps the stage blocked
for the next `war perform`.

## No responder is UNKNOWN

`war answer` accepts an actor who holds an assignment in
`docs/authority/roles.toml` and whose `actor_kind` is not `agent`. If a
repository's register names no such actor, a blocking question there can never
be answered.

So for each stage blocked by a question when the register holds no such actor,
or cannot be read, `war frontier` reports UNKNOWN `question.no-responder`
against `docs/authority/roles.toml`. The stage stays blocked. The question is
not waiting normally and it is not answered, and nothing treats it as either
(Law 15). The finding goes away once a human is added to the register.

An agent still cannot answer, and the question file is left byte-identical
when it tries (`question.agent`).

**Not yet delivered:** `war next` does not report `question.no-responder`.
`war next` carries no diagnostics, `next.rs` is outside this Warrant's
declared deliverables, and the plant "next with no responder" fails until that
changes. `war next` also still lists a question-blocked stage as executable,
because its list comes from the corpus status and not from `war frontier`.

Plants: "frontier with no responder", "next with no responder" (which fails
today), "an agent answering, no responder", and "frontier with a responder".

**Delivery.** A human learns about a question through `war watch`,
`war inbox`, `war questions --open`, `war frontier` or the UI. Nothing sends
it to someone who is not looking, such as by mail or chat. That is out of
scope (OW-WAR-0132 U-003). The UNKNOWN above is the signal, and a notifier
would be a later adapter.

## Time

A performance is bounded by the tighter of `[perform] performer_timeout_secs`
and the stage's `wall_time_seconds`. If neither is set, `[run]
default_wall_time_seconds` applies (600). At the bound, the performer's
process group is killed and the run ends `timeout` (see `docs/PERFORM.md`).

## Spend

No performer adapter meters spend. The honest state of every performance's
cost is **unknown**:

- each `perform.ended` journal event carries `"spend": "unknown"`, never `0`;
- `[perform] allow_unmetered = true` is required before `war perform` runs a
  performer that reports no spend, and today that is every performer. If the
  key is absent (or `false`), `war perform` refuses with
  `perform.unmetered-not-allowed`, and the refusal names the key to add.
  Absent is not consent. This is the default that the owner settled in
  OW-WAR-0132 U-001. Setting the key does not make the cost known; it says to
  run anyway, knowing the cost is unknown;
- `[perform] hard_spend_cap`, for example `"1.00 USD"`, refuses the run with
  `perform.spend-unenforceable` when it is set, whatever its value. Nothing
  can enforce a cap, and a mandatory limit that cannot be enforced refuses the
  execution instead of being claimed. Nothing is compiled or spawned.

This repository sets `allow_unmetered = true` in `openwarrant.toml`, with a
comment saying that its cost is unknown.

**Upgrading.** A repository that ran `war perform` before OW-WAR-0132 now
gets `perform.unmetered-not-allowed` until it adds `allow_unmetered = true`
under `[perform]` (R-001).

Plants: "a hard spend cap", "allow_unmetered absent", "allow_unmetered =
true", "spend is unknown, never zero" (over every `perform.ended` line the
plant produced), and "a zero spend would be seen" (the same check, shown
matching a line whose spend was planted as `0`).

## Attempts: repairs and recoveries

Each performance that started a performer ends by journalling
`perform.ended`. The event records the stage, `dispatch_id`, `outcome`,
`attempt`, `seconds` and `spend: "unknown"`. The outcome is one of:

| outcome | meaning |
|---|---|
| `answered` | the performer answered and the seam accepted the submission |
| `refused` | the performer answered and the seam refused the answer (§51.2, the dispatch witness) |
| `timeout` | killed at the wall-time bound |
| `cancelled` | SIGINT or SIGTERM while it ran; its answer was discarded |
| `failed` | exited without a usable submission, or flooded stdout |

A run refused before a performer is started (by any of the admissions on this
page, a writer still alive, or a Dispatch that will not compile) is not a
performance, and it journals no `perform.ended`.

The next performance of a stage is counted from its earlier endings:

- **Initial:** the stage has no ending on record.
- **Repair:** the last ending was `answered`, so this is a re-performance
  after an accepted submission. Repairs are numbered by the accepted
  submissions already on record. The first accepted submission is not a
  repair, and each accepted one after it is, so a failed run does not consume
  a repair.
- **Recovery:** the last ending was anything other than `answered`. Recoveries
  are numbered by the endings since the last accepted submission (or since the
  start) that were not accepted. An accepted submission resets that count.

A performance is refused before compiling when its number exceeds the limit:
`perform.repair-limit` past `[perform] max_repairs`, and
`perform.recovery-limit` past `[perform] max_recoveries`.

| key | default (U-001) | where the number comes from |
|---|---|---|
| `max_repairs` | 3 | the product spec's fallback of three repair cycles |
| `max_recoveries` | 2 | set by the owner in U-001 |

`0` or an absent key means the default. The stage's `attempt` (`initial`,
`repair`, `recovery`) is journalled with each ending. The Dispatch's own
`attempt_kind` is still `initial`: a repair Dispatch under §52.3 cites a
parent attempt, and linking one is not part of this Warrant.

Rebuttal accounting is not implemented. It needs the verification-repair
loop, which `war` does not have yet.

Plants: "max_recoveries = 1" (failed, failed, then `perform.recovery-limit`),
"max_repairs = 1" (answered, answered, then `perform.repair-limit`), and "a
failed run is not a repair" (with `max_repairs = 1`, answered, failed,
answered is permitted). Each checks the journal's `perform.ended` outcomes
against the runs.

## What is not counted

- **A Dispatch run by hand** (A-001). Only `war perform` journals
  `perform.ended`. A Dispatch compiled with `war dispatch` and handed to an
  agent outside `war` is not counted as a repair or a recovery, and the limits
  above do not see it.
- **Spend.** Nothing measures it, so nothing counts it. "unknown" is the
  whole record.
- **Model turns and tokens spent.** `budget_tokens` bounds the size of the
  compiled Dispatch (OW-WAR-0127), not what the performer consumes.
- **A performance whose `perform.ended` could not be journalled.** That run
  reports UNKNOWN `perform.ended-unrecorded`, and it is missing from the count
  the next run makes.

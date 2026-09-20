# Skills: the Pocock set over the deterministic core

OW-WAR-0068. Every skill under `.claude/skills/` is adapted from
[mattpocock/skills](https://github.com/mattpocock/skills) at commit 3cca18b
(MIT) with one change throughout: the artifact lands in an OpenWarrant record
the tool can read, not on an issue tracker.

| his | ours | the record it lands in |
|---|---|---|
| grilling, grill-me | `/war-grill` | `plan/request.json` answers (§74.6) |
| domain-modeling, CONTEXT.md | `CONTEXT.md` at the root | every Dispatch carries it (selector item `CONTEXT.md`) |
| ADR-FORMAT | ADR atoms may be one paragraph; identity and `governs` stay | `docs/adr/atoms/` |
| to-spec | `/war-spec` | an `oh.war/draft-proposal/v2` through the §74.4 gauntlet |
| to-tickets | `/war-tickets`, `war frontier` | `45-milestones.yaml` with `depends_on`; `oh.war/frontier/v1` |
| wayfinder | `/war-map` | a decision Warrant; fog = `blocking_unknown` assumptions |
| code-review | `/war-review` | Standards axis beside the blind verifier's Obligations axis |
| tdd | plant-first (§92), seams as obligation scopes | `conformance/plants.d/` |
| handoff | a Dispatch (§47) | `war dispatch --emit` |
| triage | `war next`, `war sign --list` | the pending set |
| writing-for-agents | applied to every SKILL.md, AGENTS.md and reference here | `cargo xtask skills` refuses an em-dash |

Two rules hold in every skill: a skill never signs, and a skill never claims
a step it did not run. `war eval` scores a skill-driven drafter on the fixed
task set, so adoption is measured, not admired.

## The frontier

`war frontier [alias]` lists every stage of every unresolved Warrant as one
of four states, derived from the records a resolution reads:

| state | meaning |
|---|---|
| `open` | its milestone's `depends_on` are complete (every obligation established) and nothing has dispatched it |
| `claimed` | a `dispatch.compiled` journal event names it, no submission yet |
| `done` | a `submission.recorded` event names it |
| `blocked` | a milestone it waits on is not complete; the row names which |

`--json` gives `oh.war/frontier/v1`.

## The hotline

A performing agent that needs a decision asks, rather than guessing or
stalling silently (OW-WAR-0069):

```bash
war ask <alias> <stage> "<question>" --recommend "<your answer>" --blocking
war questions --open                 # one queue, blocking first, with the command per row
war answer <alias> Q-001 "<answer>" --as "<human>"
war answers <alias> <stage>          # what the performer reads before it starts
war watch --once                     # questions beside pending signatures
```

`questions/Q-nnn.toml` holds each one, with `question.asked` and
`question.answered` journal events. The asymmetry is §27.2's: an agent may ask
anything and answer nothing, so `war answer` refuses an agent-kind actor by
name and writes nothing, `war_ask` is an MCP tool and `war_answer` is in the
refused list. An answer informs the work; it is never a disposition, a
judgment, or an authorization.

A question carries the asker's recommended answer, after the grilling
discipline, so a batch can be cleared in one word each.

`answered_by` is an attribution, not a proof: `war answer --as` takes a name
and checks its kind in the register, where `war sign` takes a key. That is
the right asymmetry while an answer authorizes nothing, and it is the reason
nothing downstream may read `person://…` on an answer as authority. A
malformed question record is reported by name and the rest of the queue still
lists.

## The console

One screen for everything a human owes (OW-WAR-0069):

```bash
war console                          # the checklist; 1-N toggle, a all, n none
war console --json                   # oh.war/console/v1: acts, questions, stages, presets
```

Three sections, read in one pass over the records: the acts awaiting a
signature as a numbered checklist (the same queue `war sign --list` prints),
the questions a performing agent asked, and the stages on the frontier an
agent can start now. The keys are `s` to sign the checked rows, `q` to answer
the questions, `r` to start the checked stages, `c` to print the commit
message, `x` to leave.

`s` signs nothing by itself. It runs the same `war sign --ssh-sign` a hand
would, once per checked row, so each act is still the signer's own `ssh-add
-c` confirmation. What it removes is the typing: the reason comes from a
preset the repository wrote once, plus an optional line the signer adds for
the batch, and the tool drafts the facts (which file, which digest, which
commits moved it) from the records.

```toml
# openwarrant.toml — the reasons this repository's signer reaches for
[[sign.preset]]
key = "1"
label = "a slice of the 1.0 plan"
meaning = "The delivered bytes moved with a slice of the 1.0 plan; each commit states its own change."
acts = ["correct"]
kind = "behaviour-change"
```

`acts` restricts a preset to one kind of act, and `kind` supplies `war
sign --kind` for a correction. A preset that offers itself for a correction
**must** name its kind — `war check` refuses the config otherwise — because
"the behaviour changed" and "a refusal was added" are different claims about
the same bytes, and no tool may pick one for the signer. With no preset
chosen, the console asks which, once for the batch. So twenty corrections cost
one keystroke for the rows, one for the reason, one optional sentence, and
twenty confirm dialogs — which are the twenty human acts, and the only part that
cannot be drafted.

A preset is the signer's own words, recorded verbatim. Nothing here writes a
disposition, a reason or a signature the human did not give: with no preset
chosen and nothing typed, `war sign` falls back to the reason it drafts from
the record, and the signer sees it on the confirm screen before the dialog.

## The commit message

```bash
war commit                           # print the message drafted from what changed
war commit --write                   # stage everything and commit with it
```

A commit that lands a signed act is describable without prose: the records
that appeared say which act, which Warrant, and over which digest. `war
commit` classifies every changed path (authorization, resolution, correction,
SAS revision, signed response, question, projection, journal, Warrant record,
code, docs, plant) and writes a Conventional Commits subject from the most
significant kind present — a signature outranks the projections it moved,
because the projections are its consequence. The Warrant aliases it touched
become the scope.

It will not invent a subject for changes it cannot classify, and without
`--write` it stages nothing.

## Performing a stage

```bash
war perform <alias> <stage>           # the configured performer does one agent stage
war perform --all                     # every open agent stage, one at a time
```

`war run` covered a service stage — a registered gate, a receipt, a verdict.
An agent stage had no runner: a human compiled a Dispatch, carried it to an
agent, and carried a Stage Submission back. `war perform` is that walk done by
the tool. The Dispatch goes in on the performer's stdin, its answer comes back
on stdout, and the answer is ingested through exactly the refusals `war submit`
applies to a submission that arrived by post.

```toml
[perform]
performer_argv = ["claude", "-p", "--output-format", "text"]
performer_timeout_secs = 600
max_concurrent = 1
```

What it cannot do:

- **decide the work is done.** §51.2 lets a performer ask to continue, be
  verified, block, amend or cancel. `submission.self-completion` refuses the
  rest, and a refused answer leaves nothing on disk — not the submission, and
  not the raw answer either.
- **stand in for a person.** `perform.human-stage` refuses a human stage by
  name (§27.2), and `perform.not-an-agent` sends a service stage to `war run`.
- **claim work that did not happen.** A performer that fails, or is killed at
  the bound (the tighter of `performer_timeout_secs` and the stage's
  `wall_time_seconds`), leaves the Dispatch on record and nothing else:
  `perform.failed` and `perform.timeout` each report the performer's last
  stderr line.
- **run several performers at once.** `max_concurrent` above 1 is refused by
  `perform.no-containment`: nothing here contains a performer — no cgroups, no
  sandbox — so concurrency would be several unbounded processes writing one
  tree. Raising it is a deliberate act once containment exists.

In `war console`, `r` starts the checked stages: a service stage runs its gate,
and an agent stage goes to the performer when one is configured, or has its
Dispatch compiled for you to hand over when none is. That is the "check the
boxes and an agent starts working" half of the screen; the questions it asks
you come back through the hotline.

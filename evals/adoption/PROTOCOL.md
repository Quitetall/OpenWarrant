# Adoption study protocol (pre-registered)

Status: **draft for the owner's approval.** Nothing below has been run with a
model. Written 2026-10-22 by the performer `claude` (Opus 5.5), which also built
OpenWarrant: see "Who designed this" under threats to validity.

## The question

OpenWarrant's core claim (owner direction, 2026-10-11) is that its documents are
better agent context: an agent working from them uses fewer tokens, finishes more
work correctly, resumes interrupted work better, records what it actually did,
and coordinates with other agents. Nothing has measured this. The study compares
OpenWarrant with the four ways people already give an agent a backlog, using the
same model, the same repositories and the same work.

We will report whatever comes out. If OpenWarrant is no better than a plan file,
that is the finding.

## Arms

Six arms (Jira added at the owner's request on 2026-10-22, before any pilot).

Every arm's `CLAUDE.md` starts with the same project facts (`[project] facts` in
the backlog). The backlog's words (titles, descriptions, acceptance criteria,
design notes, dependencies) are identical in every arm; only their home differs.
`run.py parity <repo>` checks that every arm holds every item's words.

| arm | where the backlog lives | what points the agent at it | tool |
|---|---|---|---|
| `plain` | a `## Backlog` section in `CLAUDE.md` | nothing more: it is in the file Claude Code reads | none |
| `plans` | `PLAN.md`, checkboxes and a handoff log | 4 lines in `CLAUDE.md` (written for the study) | none |
| `speckit` | `specs/001-backlog/{spec,plan,tasks}.md`, one task per acceptance criterion | 3 lines in `CLAUDE.md` (written for the study; Spec Kit installs skills but no `CLAUDE.md`) | Spec Kit v1.2.2, `specify init --integration claude` |
| `beads` | Beads issues (description, acceptance, design, dependencies) | the block and `SessionStart` hook `bd init` installs, verbatim | Beads v1.3.1, `bd init` |
| `openwarrant` | one ticket per item, acceptance criteria as its checklist items, dependencies as `--after` | the block `war admin agents-md --block --file CLAUDE.md` installs, verbatim | OpenWarrant 1.0.0-alpha.5, `war init` |
| `jira` | one Jira Cloud issue per item (summary, description with the acceptance criteria and notes, "Blocks" links), labelled for the cell | 5 lines in `CLAUDE.md` (written for the study): project, label, and the REST API with `curl` | Jira Cloud REST API v3 (added 2026-10-22 at the owner's request) |

The Jira arm needs credentials in the agent's sandbox (`JIRA_SITE`,
`JIRA_EMAIL`, `JIRA_API_TOKEN`, `JIRA_PROJECT`), so it should use a Jira site or
project kept for the study. Each cell's issues carry their own label and are
deleted when the cell ends. Its tracker is read from the host over the API,
since Jira's records live on Jira's servers; nothing from the repository runs
there.

Rules for fairness:

- Each tool is set up the way its own documentation sets it up for Claude Code,
  at a pinned release. We do not write weaker instructions for a competitor;
  where a tool installs its own agent instructions (Beads, OpenWarrant) they are
  used verbatim. Where a tool installs none (plan files, Spec Kit), the study
  adds the shortest pointer a user would write; its text is in `arms.py`.
- `war init` alone writes a 9 KB `AGENTS.md`. Claude Code 2.1.301 loads
  `AGENTS.md` only when a project has no `CLAUDE.md` of its own (its default,
  `claude-md-or-agents-md`). Every arm here has a `CLAUDE.md` with the project
  facts, so no arm's `AGENTS.md` loads, and the OpenWarrant arm runs the
  documented `war admin agents-md --block --file CLAUDE.md`: its agent sees the
  five-line block, which points at `war view prime` and `war context`. That is
  what an adopter with an existing `CLAUDE.md` gets.
- The seeder, not the model, writes each backlog into each tool, so no arm
  benefits from a better-drafted backlog.

## Repositories

| repo | what | language | items |
|---|---|---|---|
| `tally` | a synthetic plain-text double-entry ledger CLI, written for the study (766 lines, 37 visible tests) | Python 3, stdlib | 6 |
| `knightlm` | the owner's KnightLM chess scaffold at commit `36979e9`, read-only source (1,671 lines of Rust) | Rust (Burn, cozy-chess) | 6 |

| item | what | depends on | hidden tests |
|---|---|---|---|
| T1 | commodities on amounts, price directives, a `prices` command | | 11 |
| T2 | `balance --convert C [--at DATE]` (rounding and exit code only in the notes) | T1 | 12 |
| T3 | periodic `~` transactions and `forecast --until` | | 11 |
| T4 | `budget` report against periodic budgets | T3 | 9 |
| T5 | `import CSV --rules FILE` | | 7 |
| T6 | `balance --depth N` roll-up | | 8 |
| K1 | `perft <game> <depth> [--divide]` | | 13 |
| K2 | same-coloured bishops are a dead position | | 15 |
| K3 | `compile --legal-choices --san` | | 11 |
| K4 | `import-pgn <file> [--game N]` (history and error rules only in the notes) | K3 | 12 |
| K5 | `export-pgn <game>`, round-trips with K4 | K3, K4 | 20 |
| K6 | per-square mismatches from `verify-observation` | | 15 |

Each item has hidden acceptance tests (`repos/<repo>/hidden/<ID>/`) that no
agent can see, and a reference solution (`reference/<ID>.patch`). `run.py
validate <repo>` proves the invariants in `BACKLOG-FORMAT.md`: every check fails
on the seed, passes once its reference solution is applied, and keeps passing as
later items land. Each backlog includes design decisions stated only in the
item's notes, so an agent that never reads the item cannot pass.

## Scenarios

The prompt never names a tool. What the agent learns about the work comes from
the repository.

1. **Directed** (one session per item). Earlier items in the backlog's order are
   already done (reference solutions applied and recorded done in the arm's
   tracker). Prompt: `Do backlog item {id}: {title}.` Measures how much context
   the agent loads to start, and whether it gets the item right.
2. **Relay** (a chain of fresh sessions over the whole backlog). Prompt, every
   session: `Continue this repository's planned work: take the next item that is
   ready, finish it, and stop when that one item is done.` Session 2 is cut at
   12 turns on purpose, so session 3 must resume someone else's half-finished
   item from what the repository records. The chain stops when every item passes,
   when two sessions in a row change nothing (no file, no tracker record, no new
   pass), or after items + 2 sessions.
3. **Parallel** (three agents at once). Each works in its own git worktree of
   one repository; the same prompt adds `Other agents are working on this
   repository at the same time, each in its own git worktree.` Afterwards the
   harness merges the branches in finish order. Measures duplicated work, merge
   conflicts and how much lands.

Every session: model `claude-sonnet-5-5`, at most 80 turns (12 for relay session
2), 45 minutes, and USD 8 (API-equivalent, `--max-budget-usd`). After every
session the harness commits the tree (so arms that do not commit are not
penalised), runs every item's hidden tests, and reads the arm's tracker.

## Measures

Fixed now, before any result.

**Primary**

- **Directed success:** the share of directed sessions whose item passes its
  hidden tests, with a 95% Wilson interval per arm.
- **Relay throughput:** items passing at the end of a relay, and API-equivalent
  cost per passing item.

**Secondary**

- **Orientation cost:** context size (input plus cached tokens) of the first
  assistant turn that edits a non-tracker file, and total tokens processed
  before it. An edit is an Edit, Write or NotebookEdit call, or a Bash command
  that writes a source path (`sed -i`, a redirect, `tee`, `git apply`, a script
  that opens a file for writing); `run.py bash_writes_code` is the rule.
  (Amended before the pilot: the smoke session edited only through Bash, which
  the first version of the rule missed.)
- **False completions:** items the arm's own tracker records as done whose
  hidden tests fail (not defined for `plain`, which records nothing).
- **Unrecorded completions:** items that pass but the tracker does not record.
- **Resumption:** in relays where session 2 hit its 12-turn cut, whether
  session 3 brings a new item to passing.
- **Regressions:** earlier items that stop passing.
- **Parallel:** items passing after the merge, items solved by more than one
  agent, merge conflicts.
- Turns, wall time, tool calls, and tracker commands run.

**Analysis.** Arms are compared on the same (repository, item, replicate) cells.
We report every arm on every measure, with intervals, and no single composite
score. Differences between arms come with a paired bootstrap interval (10,000
resamples over cells). With three replicates the study can detect large
differences only; smaller ones are reported as unresolved, not as ties. No
measure is added, dropped or redefined after results are seen. If one is, it is
reported as exploratory, with the reason.

**What would count as support.** For the core claim: in both repositories,
OpenWarrant's directed success and relay throughput are at least the best other
arm's, its orientation cost is lower than `plain`'s, and its false-completion
rate is not higher than any other tracker's. Anything less is reported as the
claim being unsupported (or contradicted) on that measure.

## Isolation and safety

- Every session runs in a bubblewrap sandbox that sees only its cell directory,
  the Rust toolchain, a pinned pytest, and the pinned `claude`, `war` and `bd`.
  It does not see the owner's home, `/mnt/4tb` or `/mnt/2tb`, an SSH agent, this
  study's code, hidden tests or reference solutions. Probed on 2026-10-22.
- A fresh Claude config per session: no user `CLAUDE.md`, memory, settings,
  plugins or MCP servers (`--strict-mcp-config`, `--setting-sources
  project,local`). Permissions are bypassed inside the sandbox.
- Everything that touches a repository after an agent has (snapshot commits,
  merges, tracker reads, grading) runs in the sandbox too, without network, with
  git hooks and fsmonitor disabled. The host never executes anything an agent
  wrote.
- Raw transcripts, metrics, grades and a git bundle of each final repository
  go to `/mnt/4tb/artifacts/OpenWarrant/keep/adoption/<run-id>/`. Working trees
  and build output are deleted when each cell finishes, so disk use stays
  bounded by the cells running at once.

## Self-checks (no model)

`run.py execute` with the fixture agents (`fixture_agent.py`), on the `_selftest`
repository, across all five arms and three scenarios:

- `fixture-reference` (applies the reference solution, records it done) must
  pass every item it reaches, with no false completion.
- `fixture-noop` must pass nothing and stall.
- `fixture-claim` (records items done without doing them) must be caught as a
  false completion in every arm that has a tracker.

All three came out as required on 2026-10-22.

## Threats to validity

- **Who designed this.** The study was designed by an agent that built
  OpenWarrant, for OpenWarrant's owner. Mitigations: the backlog is tool-neutral
  and seeded mechanically; competitors use their own shipped instructions; the
  grading is mechanical hidden tests; the measures and the support rule are
  fixed above, before any run; raw transcripts are kept for anyone to re-grade.
  The owner should still read `arms.py` with an eye for an arm set up worse than
  its users would set it up.
- **Training familiarity.** The model may know Spec Kit, Beads or plain
  `PLAN.md` conventions better than OpenWarrant, which is newer. That is part of
  what adopters face, and it is not separated out.
- **Two repositories, one model, three replicates.** Results are about these
  repositories, this model and this size of work. A second model is a separate
  run.
- **Synthetic `tally`.** Written for the study; `knightlm` is a real project
  but small.
- **Pointers written for the study** (plan files, Spec Kit) are four and three
  lines; a different wording could do better or worse.

## Size

`run.py plan`: the full study is 240 cells (180 directed, 30 relays, 30 parallel)
and at most 510 agent sessions, fewer when relays finish or stall early. The
pilot is 6 cells and at most 20 sessions.

## Pilot, then the full study

1. **Smoke** (one session, cents): one directed `tally` T6 session in the
   `plain` arm, to prove the agent authenticates in the sandbox and that
   `parse_stream` reads a real transcript.
2. **Pilot** (`plan --pilot`): `tally`, arms `plain` and `openwarrant`, directed
   T1 and T2 plus one relay each, one replicate. Budget cap USD 60. It measures
   cost per session, turns and wall time, and shows whether the prompts and
   turn caps let an agent finish an item. If an item turns out to be impossible
   or trivial for every arm, it is fixed before the full study, and the change
   is recorded here; no arm-specific change follows a pilot result.
3. **Full study**: approved separately, with the pilot's measured cost per
   session in hand.

## Cost estimate (before the pilot measures it)

Sonnet 5.5 is USD 2 per million input tokens, 10 per million output, 0.20 per
million cache reads. A Claude Code session of 30 to 50 turns whose context grows
to 50k to 100k tokens costs roughly USD 0.6 to 2.5, mostly in cache reads and
writes. That puts the pilot at about USD 15 to 50 and the full study at about
USD 400 to 900. Opus 5.5 costs about twice that. On the subscription these are
the API-equivalent figures Claude Code reports; usage counts against the
plan's limits instead of being billed.

## Cost control

`run.py plan` counts sessions; `execute --budget-usd` stops starting new cells
once the reported API-equivalent cost reaches the cap; each session also has its
own USD 8 cap. The pilot runs first, and its measured cost per session gives the
estimate for the full study before the full study is approved.

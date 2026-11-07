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

Six arms are built; the first run uses five (Jira, added at the owner's request on
2026-10-22, is left for a later run, also at the owner's request).

Every arm's startup was checked from inside its sandbox (Claude Code's own `init`
event, 2026-10-22): `speckit` loads its 10 skills; `beads` fires its
`SessionStart` hook and injects `bd prime`; `openwarrant` loads its plugin, 8
skills and the connected `war mcp` server with 41 tools; `plain` and `plans`
load only their `CLAUDE.md`. No arm gets another arm's skills, hooks or servers.

Every arm's `CLAUDE.md` starts with the same project facts (`[project] facts` in
the backlog). The backlog's words (titles, descriptions, acceptance criteria,
design notes, dependencies) are identical in every arm; only their home differs.
`run.py parity <repo>` checks that every arm holds every item's words.

| arm | where the backlog lives | what points the agent at it | tool |
|---|---|---|---|
| `plain` | a `## Backlog` section in `CLAUDE.md` | nothing more: it is in the file Claude Code reads | none |
| `plans` | `PLAN.md`, checkboxes and a handoff log | 4 lines in `CLAUDE.md` (written for the study) | none |
| `speckit` | `specs/001-backlog/{spec,plan,tasks}.md`, one task per acceptance criterion, and a constitution holding the project facts (what `/speckit-constitution` leaves) | 3 lines in `CLAUDE.md` (written for the study; Spec Kit installs skills but no `CLAUDE.md`), and its 10 `/speckit-*` skills | Spec Kit v1.2.2, `specify init --integration claude` |
| `beads` | Beads issues (description, acceptance, design, dependencies) | the block and `SessionStart` hook (`bd prime`) `bd init` installs, verbatim; `bd setup claude` adds nothing further | Beads v1.3.1, `bd init` |
| `openwarrant` | one ticket per item, acceptance criteria as its checklist items, dependencies as `--after` | as QUICKSTART sets it up for Claude Code: the block `war init` appends to the existing `CLAUDE.md`, and the OpenWarrant plugin (8 skills, the `war mcp` server with 41 tools, the edit-guard and end-of-turn hooks) | OpenWarrant 1.0.0-alpha.5 and its plugin at `v1.0.0-alpha.5` |
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
- `war init` also writes a 9 KB `AGENTS.md`. Claude Code 2.1.301 loads
  `AGENTS.md` only when a project has no `CLAUDE.md` of its own (its default,
  `claude-md-or-agents-md`). Every arm here has a `CLAUDE.md` with the project
  facts, so no arm's `AGENTS.md` loads; the OpenWarrant agent reads the block
  `war init` appends to `CLAUDE.md` (pointing at `war view prime` and `war
  context`) and has the plugin's skills and MCP tools.
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
   5 turns on purpose, so session 3 must resume someone else's half-finished
   item from what the repository records. The chain stops when every item passes,
   when two sessions in a row change nothing (no file, no tracker record, no new
   pass), or after items + 2 sessions.
3. **Parallel** (three agents at once). Each works in its own git worktree of
   one repository; the same prompt adds `Other agents are working on this
   repository at the same time, each in its own git worktree.` Afterwards the
   harness merges the branches in finish order. Measures duplicated work, merge
   conflicts and how much lands.

Every session: model `claude-sonnet-5-5`, at most 80 turns (5 for relay session
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
- **Resumption:** in relays where session 2 hit its 5-turn cut, whether
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

## Amendment 1 (2026-10-29): the full study measures OpenWarrant after the pilot's fix

The pilot (`pilot-3-v2`, OpenWarrant 1.0.0-alpha.5) found the `openwarrant` arm
took more turns and tokens than `plain`, with no gain in correctness. Its owner
changed the product in response (PR #343, merged to main as `984069004`):
`war start` takes the next ready Warrant and prints only it, `war done` with no
id closes what the agent holds, `war view prime` and `war view ready` list one
line per Warrant, and an opt-in `SessionStart` hook prints the overview at the
start of a session. This is a change to the product under study, made by its
owner. The harness, backlogs, prompts, turn caps, measures and the other four
arms are unchanged.

- The full study's `openwarrant` arm runs `war` built from main at `984069004`
  (`ADOPTION_WAR`) and the plugin from the same commit (`ADOPTION_OW_PLUGIN_REF`),
  both recorded in `run.json`. It is set up as shipped: the session hook is
  off, since `[tickets] session_start` is opt-in and QUICKSTART does not turn it on.
- The pilot's alpha.5 runs stay the before-measure.
- An exploratory run, `full-1-hook`, repeats the `openwarrant` arm with the hook
  on (`ADOPTION_OW_SESSION_START=1`), one replicate. It is reported beside the
  study, not as one of its comparisons.
- Cost: the pilot measured USD 0.13 to 0.18 per Sonnet 5.5 session, not the
  0.6 to 2.5 estimated above. With the v2 backlogs (11 items each), the full
  plan is 390 cells (330 directed, 30 relays, 30 parallel) and at most 810
  sessions. Caps: USD 125 for the study and USD 15 for the exploratory run, out
  of about USD 185 left on the key; the rest is kept for a re-measure after
  further changes. Cells run replicate by replicate with arms interleaved, so a
  budget stop leaves every arm with the same cells, and the report says which ran.

## Amendment 2 (2026-10-29): a second agent, opencode

At the owner's request the harness also runs opencode 2.2.1 (`--agent opencode`)
with models from the LithosAI endpoint (`--model lithosai/deepseek-ai/DeepSeek-V4.1-Flash`
or `lithosai/zai-org/GLM-5.3-Flash`), so a change can be checked for agents other
than Claude Code. Claude Code stays the study's agent; opencode runs are reported
as their own runs, never pooled with it.

- opencode 2.2.1 reads `AGENTS.md` (and `~/.config/opencode/AGENTS.md`) at startup
  and never `CLAUDE.md`, as its debug log showed on 2026-10-29. Under opencode,
  each arm keeps the project's instructions in `AGENTS.md` instead, and each tool
  is set up as its own documentation sets it up for opencode:
  - `plain`, `plans`, `jira`: the same text, in `AGENTS.md`.
  - `speckit`: `specify init --integration opencode`, which installs its ten
    commands in `.opencode/commands/`. The pointer names `/speckit.implement` and its
    file, since an agent cannot invoke a user's slash command.
  - `beads`: `bd init` appends its block to `AGENTS.md`; `bd setup opencode`, the
    documented step, adds nothing further.
  - `openwarrant`: `war init` appends its block to `AGENTS.md`, and QUICKSTART's
    setup for "any other harness" adds `war mcp` as an MCP server. OpenWarrant has
    no opencode plugin, so there are no skills or hooks.
- Sessions run in the same sandbox. Every tool is allowed by the session's
  `permission` config, the sandbox being the boundary, as with Claude Code's
  `bypassPermissions`. The turn cap is the build agent's `steps`. The provider key
  reaches the sandbox on a file descriptor, never in an argv or the environment.
  Autoupdate and the models.dev fetch are off.
- Metrics come from opencode's JSON events: one step per model call, carrying that
  call's input, cache and output tokens. The endpoint reports no price, so opencode
  runs are compared on turns and tokens, not USD.
- `run.py report` now adds turns and thousands of tokens per session to every
  row, for both agents.

## Amendment 3 (2026-10-30): opencode's openwarrant arm without `war mcp`

Under opencode the `openwarrant` arm first had `war mcp` as an MCP server, as
QUICKSTART then told "any other harness". The opencode runs showed that path
costing three turns and about 20 KB before every brief: opencode loads MCP tools
lazily (the first call fails, a tool search follows), and in its code mode a tool
answers with the structured JSON envelope. On 2026-10-30 QUICKSTART changed to
"a harness with a shell needs nothing more" than the block, and `war mcp` became
the setup for a harness without one. From then, opencode runs set the arm up that
way: no MCP server. `ADOPTION_OC_OW_MCP=1` restores the earlier setup, and
`run.json` records which was used (`opencode_ow_mcp`). Runs before this amendment
(`oc-smoke-*`, `oc-base-1-*`, `oc-base-2-*`) had the server.

## Amendment 4 (2026-11-06): re-measuring the openwarrant arm on main

After the results above, OpenWarrant changed: terse output, `war context` on real
documents, one list of work, live notices, and a brief before the first turn
(#380 to #389). To see what they did, runs `oc-rem-*` repeat the seven opencode
cells of `plan-oc-ow.json`, unchanged, with `ADOPTION_WAR` set to a release build
of main at 1203092ac. Nothing else changes: not the seed, the prompts, the turn
cap or the measures. `--ready` is not used. `run.json` records the build.

`run.py prepare` installs neither Beads nor Spec Kit, and the study's copies aged
out of scratch. `ADOPTION_ONLY_OPENWARRANT=1` skips those two tool checks for a run
whose plan has only the openwarrant arm. It changes no cell.

The baselines ran on other days, so a difference is suggestive, not a controlled
comparison, and these seven cells are few.

Runs `oc-rem-2-*` measured main at 1203092ac. They found sessions reading every
ticket file directly, so #390 changed the block's wording, and `oc-rem-3-*`
measured that alone. Run `rem-c2` then repeats the 26 Claude cells of
`plan-full-1-hook.json` on main at 2596016db, with the plugin from the same commit
(`ADOPTION_OW_PLUGIN_REF`), so it has the prompt hook. `full-1` and `full-1-hook`
are its comparisons. Model, seed, prompts, turn cap and measures are unchanged.

Run `rem-c2` passed 20 of 22 directed cells, both failures false completions, where
the earlier runs passed all of theirs. Rerunning only those two would flatter the
result, so `rem-c3` repeats all 22 directed cells as rep 2, on the same build and
plugin. This was decided after seeing `rem-c2` and before seeing `rem-c3`.

`rem-c3` failed the same two items, and the cause was found: sections of a single-file
ticket never reached a brief (#391). `rem-c4` repeats the 22 directed cells as rep 3 on
main at 0d78465442daf0b6b96b0b5a446f3651e9f65adb, with that fix and the plugin from the same commit.

In `rem-c4` all 11 knightlm cells failed before any test ran: the grader's offline
cargo registry was empty (`no matching package named anyhow`), so they say nothing
about the agents and are excluded. The cause of the emptied registry wasn't found. Its
tally cells stand. `rem-c5` repeats the 11 knightlm cells on the same build, after
`prepare` restored the registry. This was decided before seeing `rem-c5`.

### Results of amendment 4

OpenWarrant arm on Sonnet 5.5, directed cells, main at 0d7846544: `rem-c4` tally and
`rem-c5` knightlm.

| Run | Cells | Passed | False completions | Mean turns | Cost per 22 | Median tokens before first edit |
|---|---|---|---|---|---|---|
| `full-1` | 44 | 44 | 0 | 13.5 | 3.74 | 142,166 |
| `full-1-hook` | 22 | 22 | 0 | 11.9 | 3.65 | 132,039 |
| `rem-c2` and `rem-c3` | 44 | 40 | 4 | 10.5 | 3.24 | 101,595 |
| fixed | 22 | 22 | 0 | 10.4 | 3.19 | 100,883 |
| `plain` in `full-1` | 44 | 44 | 0 | 9.9 | 3.92 | |

The comparisons ran on other days and against other builds, so these are suggestive.
Relay and parallel weren't repeated after the fix: `rem-c2` cost 1.89 and 0.88 against
3.28 and 1.26 before it. The study's value here was the four false completions, which
found a bug no test had: sections of a single-file ticket never reached a brief.


## Amendment 5 (2026-11-07): OpenWarrant versions as arms

Written before any run under it. Amendment 4 compared builds run on different days, so
its differences were suggestive only. From here, at the owner's direction, OpenWarrant
versions are arms of one run: they run side by side, on the same day, on the same
cells, and the comparison between them is controlled. The result decides which `war`
the owner uses: if alpha.7 beats alpha.6, the owner's `war` moves to alpha.7, otherwise
it stays on alpha.6; if `next` beats alpha.7, `next` is released as alpha.7.1.

**Arms.** Each is the `openwarrant` arm, seeded by the same code; only the `war` binary
in the sandbox and the plugin ref differ (`run.py` `OW_VERSIONS`).

| arm | `war` | sha256 | plugin ref |
|---|---|---|---|
| `openwarrant@a6` | the published release, `~/.local/lib/openwarrant/1.0.0-alpha.6/bin/war` (reports `war 1.0.0-alpha.6 (unreleased, commit 9cbe852edd4d)`, the tag's commit) | `ad3cbaf2c15368e62efc75191525b59a9c9012144ca8f7f241faa54a6193e72d` | `v1.0.0-alpha.6` |
| `openwarrant@a7` | the published release, `~/.local/lib/openwarrant/1.0.0-alpha.7/bin/war` (`war 1.0.0-alpha.7`, tag at `9e4948385`) | `1200b197f95f5dc8386748f71d855531a223c454cb61dd6b64966fdb60353722` | `v1.0.0-alpha.7` |
| `openwarrant@next` | a local build of main, whatever main is when it runs (`ADOPTION_WAR_NEXT`) | recorded | the commit it was built from (`ADOPTION_OW_PLUGIN_REF_NEXT`) |

`run.json` (now also written to the keep store, with the plan) records each arm's binary
path, version string and sha256 and its plugin ref, and every cell's `result.json`
records its own arm's. The unversioned `openwarrant` arm is unchanged for old runs; its
pin moves to alpha.7 (`PINS`), which is also the `war` the other arms' sandboxes hold.
Under opencode no plugin loads (amendment 2); the ref is recorded all the same.

**Seeding is the same.** Checked before any run: tally's directed T8 cell (T1 to T7
done, opencode layout) seeded for `openwarrant@a6`, `openwarrant@a7`, and the old
`openwarrant` arm with `ADOPTION_WAR` set to alpha.7. Ticket ids, item ids, uuids,
timestamps, hashes, commits and the seed path differ on every seeding, even with one
binary, so they were named first (ticket ids by the item each holds). Then the old arm
on alpha.7 and `openwarrant@a7` are identical, and alpha.6 against alpha.7 differ only
in what `war init` itself writes: the managed block in `AGENTS.md` (alpha.7 says to run
`war start` before reading anything else, and not to read `docs/tickets/` directly),
two `merge=war-ticket` lines in `.gitattributes`, and alpha.6's `schemas/` directory.
Every ticket and journal is the same.

**Held fixed across arms:** the seed, the prompts, the agent (opencode 2.2.1), the model
(`lithosai/deepseek-ai/DeepSeek-V4.1-Flash`), the turn cap, and the measures. The turn
cap is 60 for every arm in these runs (`execute --turns 60`), not the 25 of the earlier
opencode runs, so these runs are not compared with those. Relay session 2 stays cut at 5.

**Reported, not a new measure.** Each directed row adds the median number of turns
before the first code edit (`turns_before_first_code_edit`, from the same transcripts
and the same edit rule as orientation cost), sessions that never edited code, and median
tokens per cell. No measure is changed.

**Win rule, fixed now, the same for every pair** (`run.py` `win_rule`; `report` prints
each step's numbers). It is applied to the directed cells both arms ran; relay cells are
reported beside it.

1. False completions first: a version with more loses, whatever else it does.
2. Then passes: more hidden-test passes wins. A difference of one cell or less is a tie.
3. On a tie, median tokens per cell (input, cache reads and writes, output): at least
   10% fewer wins; within 10% is a tie.
4. A tie overall is no win: the owner stays on the current version.

A cell that errored for harness reasons, not the model's (the endpoint refusing, an
emptied registry, a grade that never ran), is rerun once for every arm, or excluded for
every arm, and the report says which. Pairs: `openwarrant@a7` against `openwarrant@a6`
now, `openwarrant@next` against `openwarrant@a7` once alpha.8 work lands on main.

**First run, `oc-v5-1`.** The 14 cells of `plan-oc-base.json` (tally T8 to T10, knightlm
K7 to K9, one tally relay) on `plain`, `openwarrant@a6` and `openwarrant@a7`: 21 cells,
arms interleaved per item, `--jobs 3`, under `systemd-run --user`. The endpoint was up
and reported no price on 2026-11-07, as on 2026-10-29. No Claude cells: they cost money
and wait for the owner. `openwarrant@next` waits for alpha.8 work; until then it is
proved only by fixture runs on a build of main at `9e4948385`, which is alpha.7's own
commit. Fixture runs (`fixture-reference`, `fixture-claim`) on tally T8 already showed
`openwarrant@a6` and `openwarrant@a7` seed, grade and read their trackers: the reference
passes in both, and a claimed but undone item is a false completion in both.

### Results of amendment 5: `oc-v5-1`

Run 2026-11-07 on harness commit `45ed987f1`, 21 cells, no harness errors (every cell's
grade ran, no session errored), so nothing was rerun or excluded. From `run.py report`:

| arm | directed passed | false completions | turns per session | ktokens per session | median ktokens per cell | median turns before first edit | relay items passing |
|---|---|---|---|---|---|---|---|
| `plain` | 6 of 6 | 0 | 34.5 | 1,822.9 | 1,730.3 | 7.0 | 11, in 12 sessions |
| `openwarrant@a6` | 6 of 6 | 0 | 56.0 | 4,561.2 | 4,886.3 | 10.0 | 1, stalled after 3 |
| `openwarrant@a7` | 6 of 6 | 0 | 38.3 | 1,636.1 | 1,980.3 | 8.5 | 11, in 12 sessions |

`openwarrant@a7` against `openwarrant@a6`, by the rule: false completions 0 and 0;
passes 6 and 6, a tie; median tokens per cell 1,980,342 against 4,886,330, 59.5% fewer.
**alpha.7 wins on tokens.** Six directed cells and one relay per arm, one replicate:
the token difference is large, and the pass counts can't separate the arms.

`openwarrant@next` was not run. Fixture runs (`fixture-reference`, `fixture-claim`) on
tally T8 proved the arm on a build of main at `9e4948385` (sha256
`bd86c70c4242f3f58c23113614afb5fa3119b0a2f627896fb52e2b1e2da941c5`; it reports `war
1.0.0-alpha.7 (unreleased, commit 9e4948385513)`, and differs in bytes from the
published alpha.7): the reference passed, the claim was a false completion, and its
seed is identical to `openwarrant@a7`'s once per-seeding ids are named.

### Results of amendment 5: `oc-v5-2`, `next` against alpha.7

Run 2026-11-07, the same day as `oc-v5-1`, on harness commit `781cf04f2`: the same 21
cells with `openwarrant@a6` replaced by `openwarrant@next`, a release build of main at
`f028d53da` (#403 to #407; reports `war 1.0.0-alpha.7 (unreleased, commit
f028d53da687)`, sha256 `9d1177d3e43a863d05c0c7bb1e72bc41294442bb985d825e22c8e99de015f4be`,
plugin ref `f028d53da`). No harness errors; nothing rerun or excluded.

Seeding checked first, as for `oc-v5-1`: once per-seeding ids are named, `next` and
alpha.7 differ only in the `AGENTS.md` block `war init` writes. One trap found: run from
a Claude Code shell, `next`'s `war done` adds a `work.usage` event (`source:
claude-code`) after each item the seeder closes, because the seeder passes the shell's
`CLAUDE_CODE_SESSION_ID` through. The `systemd-run` unit that runs the study has no such
variable (checked), `oc-v5-1`'s kept seed records only `local-session:<pid>`, and
without it the journals are identical. Fixture runs showed `next`'s tracker read
correctly (reference passed and recorded; a claim was a false completion).

| arm | directed passed | false completions | turns per session | ktokens per session | median ktokens per cell | median turns before first edit | relay items passing |
|---|---|---|---|---|---|---|---|
| `plain` | 6 of 6 | 0 | 33.3 | 1,802.7 | 1,793.2 | 7.0 | 11, in 12 sessions |
| `openwarrant@a7` | 6 of 6 | 0 | 41.0 | 1,752.9 | 1,793.0 | 8.5 | 11, in 12 sessions |
| `openwarrant@next` | 6 of 6 | 0 | 41.8 | 1,896.2 | 1,850.7 | 9.0 | 11, in 12 sessions |

`openwarrant@next` against `openwarrant@a7`, by the rule: false completions 0 and 0;
passes 6 and 6, a tie; median tokens per cell 1,850,724 against 1,792,970, `next` 3.2%
more, inside 10%. **A tie: no win.** By the owner's rule `next` is not released as
alpha.7.1 on this result. For scale: alpha.7's own median per cell was 1,980,342 in
`oc-v5-1` and 1,792,970 here, a 9.5% swing between two runs of one binary on one day.

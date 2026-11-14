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

### Setup note to amendment 5: the seeder strips session variables

Made after `oc-v5-2`, with no run live. `arms.seed` and `arms.scaffold` now hand `war`
(and every other seeding tool) the caller's environment less its session and harness
variables: every `CLAUDE*` and `CODEX_*` variable, `OPENWARRANT_SESSION_ID`,
`KATANA_SESSION_ID`, and, as before, the SSH agent's. A seed made by hand from an agent's
shell now matches one made by a run's `systemd-run` unit. It changes no cell of `oc-v5-1`
or `oc-v5-2`: neither unit's environment had any of these variables. Checked from a
Claude Code shell after the change: tally's T8 cell seeded for `openwarrant@a7` and
`openwarrant@next` (main at `f028d53da`) is identical once per-seeding ids are named,
except the `AGENTS.md` block `war init` writes, and no journal holds a `work.usage` event
or a Claude session.

## Amendment 6 (2026-11-07): two more replicates, and opencode with `war mcp`

Written and pushed before any run under it, at the owner's direction.

**`next` moves to main at `d9b524f5c`** (#409, the instruction-block fix). Release build;
it reports `war 1.0.0-alpha.7 (unreleased, commit d9b524f5c0ef)`, sha256
`6f09c3d2349c6071b90dbaad076bdf98f7ef80efdd04db740a88cb46f83720cc`, plugin ref
`d9b524f5c`. Every `@next` arm in `oc-v5-3` and `oc-v5-4` uses it. `oc-v5-2`'s `next` was
`f028d53da`; it is pooled with these as the same arm, as the owner directed.

**New arms, opencode only.** `openwarrant-mcp@a7` and `openwarrant-mcp@next` are
`openwarrant@a7` and `openwarrant@next` with `war mcp` connected: the same build, seed,
`AGENTS.md`, prompts, model and turn cap, and one difference, the session's opencode
config. Its `mcp` section registers a local stdio server, `{"openwarrant": {"type":
"local", "command": ["war", "mcp"], "enabled": true}}` (`run.py` `opencode_config`), with
`war mcp`'s default tool set and no flags. opencode starts it in the session's directory,
where `war mcp` finds the repository. opencode 2.2.1 reaches MCP tools through its code
mode: the agent searches a namespace and calls `tools.openwarrant.<tool>` inside its
`execute` tool. (`opencode mcp list` prints "No MCP servers configured" under the same
config, though `opencode debug config` shows the server parsed and sessions use it; that
command doesn't show what a session loads.)

Proved before the runs, in the study's sandbox for tally's T8 cell, with one short
session per build on the endpoint (not a study cell): asked to list the server's tools
and call one, the agent found them by `search` and called them through `execute`.
- `openwarrant-mcp@a7`: 42 tools listed; `war_next` returned a well-formed `oh.war/report/v1` envelope.
- `openwarrant-mcp@next`: 7 tools listed (#406's lean default: `war_check`, `war_context`,
  `war_create`, `war_done`, `war_note`, `war_start`, `war_status`); no `war_next`, so it
  called `war_status({next: true})`, which returned the ready list.

A plain MCP client in the same sandbox, with no model, also listed alpha.7's 42 tools and
called `war_next`. Seeds, with per-seeding ids named: `openwarrant@next` and
`openwarrant-mcp@next` are identical; alpha.7 and `next` differ only in the `AGENTS.md` block.

**Runs.** `oc-v5-3`, then `oc-v5-4`: each one `systemd-run` unit holding all five arms,
`plain`, `openwarrant@a7`, `openwarrant@next`, `openwarrant-mcp@a7` and
`openwarrant-mcp@next`, on the 7 cells per arm of `oc-v5-2` (35 cells, arms interleaved per
item, `plan-oc-v6.json`), at 60 turns, `--jobs 3`, after checking the endpoint is free and
the grader's registry holds crates. Seeding goes through `arms.seed_env()`.

**Pooling, fixed now** (`run.py pool <run-id>...`). For each pair, amendment 5's rule over
the directed cells of every run holding both arms: `openwarrant@next` against
`openwarrant@a7` over `oc-v5-2`, `oc-v5-3` and `oc-v5-4`; `openwarrant-mcp@next` against
`openwarrant-mcp@a7` over `oc-v5-3` and `oc-v5-4`. False completions and passes are
summed; passes tie when they differ by no more cells than the number of replicates
pooled. Tokens are the median over the pooled cells, with the 10% band. Each MCP arm is
also shown beside its twin (`openwarrant-mcp@X` beside `openwarrant@X`): passes, false
completions, median tokens per cell, turns and turns before the first edit, with no win rule.

**A harness error, defined now.** The machine is shared, and at a load average near 180 on
2026-11-07 opencode twice stalled at startup and produced no event at all before a timeout.
A session whose transcript holds no model step is a harness error, not the agent's
result. Amendment 5's rule applies: the cell is rerun once for every arm, or excluded for
every arm, and the report says which.

### Results of amendment 6: `oc-v5-3`

Run 2026-11-07 on harness commit `74e8900c8`, 35 cells, the five arms above, `next` at
`d9b524f5c`. No harness error: every session's transcript holds model steps and every
grade ran (one knightlm `plain` session exited 1 after 39 turns, the agent's own end), so
nothing was rerun or excluded. Its relay cells are affected by the leak amendment 7 fixes:
read that before the relay column. From `run.py report oc-v5-3`:

| arm | directed passed | false completions | turns per session | ktokens per session | median ktokens per cell | median turns before first edit | relay items passing |
|---|---|---|---|---|---|---|---|
| `plain` | 6 of 6 | 0 | 42.2 | 2,421.3 | 2,696.7 | 7.0 | 11, in 12 sessions |
| `openwarrant@a7` | 6 of 6 | 0 | 39.5 | 1,691.6 | 1,823.3 | 9.0 | 11, in 13 sessions (T8 in session 13, from leaked grades) |
| `openwarrant@next` | 6 of 6 | 0 | 39.5 | 1,845.6 | 2,109.9 | 8.5 | 10, in 13 sessions; T8 a false completion at the end |
| `openwarrant-mcp@a7` | 6 of 6 | 0 | 39.5 | 2,260.6 | 2,253.1 | 9.0 | 11, in 13 sessions (T8 in session 13, from leaked grades) |
| `openwarrant-mcp@next` | 6 of 6 | 0 | 45.2 | 2,162.8 | 2,123.0 | 10.0 | 11, in 13 sessions |

By the rule, in this run alone: `openwarrant@next` against `openwarrant@a7`, 0 and 0 false
completions, 6 and 6 passes, median tokens per cell 2,109,904 against 1,823,340, `next`
+15.7%: alpha.7 wins on tokens. `openwarrant-mcp@next` against `openwarrant-mcp@a7`: 0 and
0, 6 and 6, 2,122,966 against 2,253,126, −5.8%: a tie.

**Pooled** (`run.py pool oc-v5-2 oc-v5-3`, ties at two cells for the first pair, one for the
second):

| pair | runs | cells each | false completions | passes | median tokens per cell | result |
|---|---|---|---|---|---|---|
| `openwarrant@next` against `openwarrant@a7` | `oc-v5-2`, `oc-v5-3` | 12 | 0 and 0 | 12 and 12 | 2,019,327 against 1,823,340, +10.7% | tie: no win |
| `openwarrant-mcp@next` against `openwarrant-mcp@a7` | `oc-v5-3` | 6 | 0 and 0 | 6 and 6 | 2,122,966 against 2,253,126, −5.8% | tie: no win |

Beside their twins (descriptive, `oc-v5-3`): `openwarrant-mcp@a7` 2,253,126 median tokens per
cell against `openwarrant@a7`'s 1,823,340, the same 39.5 turns per session and 9.0 turns
before the first edit; `openwarrant-mcp@next` 2,122,966 against `openwarrant@next`'s
2,109,904, 45.2 turns per session against 39.5, 10.0 turns before the first edit against 8.5.

**What a diagnosis of `oc-v5-3` found**, recorded here with no change to any measure:

- *The pooled token gap is noise at this size.* `next` against alpha.7 is +10.7% as a ratio
  of medians, just outside the 10% band, but paired cell by cell (the geometric mean of the
  12 per-cell ratios of `next` to alpha.7 on the same item and run) it is +0.8%, bootstrap
  95% interval −22% to +29%. That is within the spread of one binary between runs: alpha.7's
  own median per cell was 1,980,342 in `oc-v5-1`, 1,792,970 in `oc-v5-2` and 1,823,340 here.
  The rule's verdict stands as written, a tie, and the paired figure says nothing larger is
  hidden behind it.
- *The relay false completion is the model's, not `next`'s.* `openwarrant@next` ended with T8
  recorded done and failing. Both alpha.7 relays made the same false completion (T8 recorded
  done and failing from session 10 in one, 11 in the other); they ended clean only because session 13 in each read `/work/s01` to `/work/s12/grade.json`, the
  hidden tests' failure output, and fixed T8 from it. That is the leak amendment 7 closes;
  without it the three relays would not differ on T8.
- *One cost was `war`'s.* `next`'s sessions read `docs/tickets/` directly more often than
  alpha.7's, where `war start` would have given them the ticket. It traces to the wording of
  the `AGENTS.md` block #406 introduced, which is reverted in PR #416.

## Amendment 7 (2026-11-12): a leak between sessions, closed

Written and pushed before any further run, after a diagnosis of `oc-v5-3`.

**The leak.** "Isolation and safety" says a session sees only its cell directory. It did,
and that was the hole: `Sandbox.argv` bound the whole cell read-write at `/work`, and the
harness wrote each session's records into the cell, at `sNN/` (`aN/` and `merged/` for
parallel cells): the transcript (`stream.jsonl`), `metrics.json`, `tracker.json`, and
`grade.json`, whose `tail` fields hold the hidden tests' failure output. So every relay
session after the first could read every earlier session's grade and transcript at
`/work/s01/...`. The hidden tests themselves, the reference patches and `backlog.toml` were
never reachable from an agent's session (they are mounted only for grading and for fixture
agents, and every attempt in the transcripts to list `/opt/hidden` failed), but their
failure output was, one session late. A directed cell has one session, so it had nothing
earlier to read.

**Who read it.** Every kept transcript of every run (Claude and opencode) was searched for
a tool call naming `grade.json`, `tracker.json`, `metrics.json`, a session directory
(`/work/sNN`, `/work/aN`, `/work/merged`), `/opt/hidden` or `repo-def`, and every tool
output for a grade's contents or a listing of a session directory. Both searches find the
same cells, all opencode relays of tally:

| run | cell | session | what it read |
|---|---|---|---|
| `oc-v5-1` | relay, `openwarrant@a6` | s03 | s01 and s02's `grade.json`, `tracker.json`, `metrics.json` and transcripts |
| `oc-v5-3` | relay, `openwarrant@a7` | s13 | s01 to s12's `tracker.json` and `grade.json`, then T8's failure tails in s01 and s07 to s12 |
| `oc-v5-3` | relay, `openwarrant-mcp@a7` | s13 | s12's prompt, grade, tracker, metrics and transcript, s11's T8 grade, then every session's grade |
| `oc-rem-2-deepseek` | relay, `openwarrant` | s04, s05 | earlier sessions' prompts, trackers, metrics and grades (s05 also s01's transcript) |
| `oc-rem-3-glm` | relay, `openwarrant` | s09 | s01 to s08's grades, s08's tracker and metrics |

(`oc-rem-2-glm`'s relay session s07 read its own prompt and the tail of its own transcript,
under `/work/s07`; nothing earlier.) No directed cell and no parallel cell, in any run, read
a session record, and no Claude session did. `oc-v5-2` has no such read.

**Effect.** Relay results that depended on earlier grades aren't comparable with relays that
didn't: in `oc-v5-3`, both alpha.7 relays' T8 (amendment 6's results: fixed in s13 from the
leaked tails, where `next`'s relay, which read none, ended with T8 a false completion), and
`oc-v5-1`'s alpha.6 relay, which read grades in s03 and stalled anyway. The relay rows of
those cells stand as recorded but are marked as leaked; the relays of `oc-v5-2` and the
other cells of `oc-v5-1` and `oc-v5-3` are unaffected. Directed cells, and so every win-rule
verdict and every pool, are unaffected. No measure changes.

**The fix** (`run.py` `workspace`). A sandbox now mounts only `<cell>/work` at `/work`:
the repository, the parallel worktrees, build output, and a spec author's own brief and
capture directories. The harness writes the records of each session (`sNN/`, `aN/`,
`merged/`), `result.json`, `seed-head` and a spec author's `questions.jsonl` beside it in
the cell, which no sandbox mounts. Grading still mounts the hidden tests read-only, only
while it runs, and writes its grade from the host. `keep_cell` keeps what it kept before,
plus a fixture author's `probe.json`, never a file an agent wrote except through the
repository's bundle, and deletes the workspace when the cell ends.

**What stays visible, decided.** A sandbox still mounts the host's `/usr` read-only, and
`find /` in a session lists it: python, cargo, git and the agents' runtimes live there. It
holds nothing of the study's (checked on the host: no file under `/usr` or `/etc` is named
for OpenWarrant, the study, tally or knightlm; and the probe below walks all of `/usr` for
the names of every hidden test and reference patch, and finds none), so it stays. `/etc` is
the same. Two mounts are shared between cells, read-write, as before: the offline cargo
registry (`/opt/cargo-home`) and the compiler cache (`/opt/sccache`). They hold crates and
compiled objects of agents' code (the hidden tests are Python and never compiled), not
grades, hidden tests or reference solutions, so they stay as they are; they are named here
because in principle a session could write to them.

**Proved, no model** (`run.py isolation-check`, scratch roots only). A scripted session
(`fixture_probe.py`) that mounts nothing a real agent doesn't, except itself, ran as a tally
relay on `openwarrant@a7`. It changes nothing, so the relay stops after two sessions, both
graded. In session 2 it tried to stat and read earlier sessions' records under `/work`, the
cell on the host, the study's `hidden/`, `reference/` and `backlog.toml` on the host, the keep
store and runs root, the harness's code, `/opt/hidden`, `/opt/study/repo-def`,
`/opt/study/code`, `/mnt` and the owner's home; walked the whole filesystem for a file named
as a hidden test, a reference patch, `backlog.toml` or a session record; and read every
small file under `/work`, `/tmp`, `/run`, `/home` and `/opt` for a grade's keys. Each attempt
failed and each search found nothing. The refusal: the same probe with the cell mounted at
`/work` as before, and the grader's and a fixture's mounts, reached `/work/s01/grade.json`,
`/opt/hidden` and `/opt/study/repo-def` and found the hidden tests by name. Then a
`fixture-reference` relay on the same arm passed all 11 items in 11 sessions with no false
completion, every session's grade kept: grading works as before. 6 of 6 checks passed on
2026-11-12.

## Amendment 8 (2026-11-12): tokens judged by a paired ratio, a criterion against plain, and what `next` now is

Written and pushed before any further run, at the owner's direction. No run was started
and no model was called to write it.

**Why.** Amendment 5's token step compares the two arms' medians of total tokens and calls
anything inside 10% a tie. Run to run, the same binary moves by about that much: alpha.7's
median per cell was 1,980,342 in `oc-v5-1` and 1,792,970 in `oc-v5-2`, 9.5% apart, and
paired cell by cell `next` against alpha.7 was +0.8% with a 95% interval of −22% to +29%
(amendment 6's diagnosis). A 10% band on a median of six or twelve totals mostly reports
ties, and when it reports a win it can be noise: `oc-v5-3` alone gave alpha.7 the win at
+15.7% on medians, where the same six cells paired give −8.7% (−38.3% to +30.0%).

**The token step, from this amendment on** (`run.py` `paired_ratio`, `win_rule`). Steps 1,
2 and 4 of amendment 5's rule are unchanged, with amendment 6's pooling: false completions
first, then passes with a tie of one cell per replicate pooled, and a tie overall is no win.
Step 3 becomes:

3. On a tie in passes, tokens are judged by the paired ratio. For every directed cell both
   arms ran (the same item, in the same replicate, of the same run), take the new
   version's tokens over the old version's (input, cache reads and writes, output, as
   before). The statistic is the geometric mean of those ratios over all pooled paired
   cells, with a bootstrap 95% interval. The new version wins on tokens only when the whole
   interval is below 1.0. It loses on tokens when the whole interval is above 1.0.
   Otherwise the token step is a tie.

The interval is a percentile bootstrap: the pairs are resampled with replacement 100,000
times, the mean log ratio taken each time, and the 2.5th and 97.5th percentiles read off;
the seed is fixed (20261022, the harness's), so a report is the same every time it is made.
A pair in which either cell reports no tokens has no ratio; it is left out of the token
step and counted in the report. A cell counts whether or not it passed: passes are step
2's business. One pair has no interval, and is a tie.

**Earlier verdicts stand as recorded.** Every verdict in the results of amendments 5 and 6
was reached by the median step and is not reopened. `run.py report` and `run.py pool` now
print both figures for every pair, the median step's and the paired step's, and both
verdicts: "Result" follows this amendment, and the line under it says what amendment 5's
step gives. A report regenerated for an old run therefore shows a second verdict beside
the recorded one; the recorded one is the one that counted.

**Checked against the recorded runs** (`run.py pool oc-v5-2 oc-v5-3`, on a scratch copy of
the kept results; the keep store was not rewritten):

| pair | cells | median step (amendment 5) | paired ratio (this amendment) | verdict by each |
|---|---|---|---|---|
| `openwarrant@next` against `openwarrant@a7`, `oc-v5-2` and `oc-v5-3` | 12 | 2,019,327 against 1,823,340, +10.7% | 1.008: +0.8%, interval −22.1% to +28.6% | tie, tie |
| `openwarrant-mcp@next` against `openwarrant-mcp@a7`, `oc-v5-3` | 6 | 2,122,966 against 2,253,126, −5.8% | 0.933: −6.7%, interval −16.5% to +3.5% | tie, tie |
| `openwarrant@a7` against `openwarrant@a6`, `oc-v5-1` | 6 | 1,980,342 against 4,886,330, −59.5% | 0.322: −67.8%, interval −80.7% to −50.8% | alpha.7 wins, alpha.7 wins |

The first row is the diagnosis's figure: the same +0.8%. Its interval was given as −21.7% to
+28.5%, from 10,000 resamples of another seed; 100,000 resamples of this one give −22.1% to
+28.6%. The difference, 0.4 of a point at the lower end, is resampling noise: 10,000
resamples under six seeds put the lower end between −22.2% and −21.7%. The seed was not
searched for one that matches.

**A criterion against plain** (`against_plain`; in `report` and in `pool`). Each OpenWarrant
arm is also set against `plain` on the directed cells both ran, by the same paired ratio,
with passes and false completions beside it. This is descriptive: it has no win rule and
decides nothing between versions. The owner's release criterion reads one row of it:
**`openwarrant@next`'s ratio against `plain` at or below 1.0**, the point estimate, shown
with its interval. Over `oc-v5-2` and `oc-v5-3` (12 cells): `openwarrant@next` 0.891
(−10.9%, −26.2% to +7.7%), `openwarrant@a7` 0.884 (−11.6%, −29.8% to +13.8%); over
`oc-v5-3` (6 cells) `openwarrant-mcp@next` 0.889 and `openwarrant-mcp@a7` 0.953; over
`oc-v5-1`, `openwarrant@a6` 2.515 (+96.7% to +239.2%). Twins (amendment 6) get the same
ratio printed under their table.

**What `next` now is: a default test.** The product is gaining `[tickets] done_test =
"<command>"` in `openwarrant.toml`: `war done` runs it when a ticket declares no test of
its own, and refuses to close the ticket when it fails. For the `@next` arms only
(`openwarrant@next`, `openwarrant-mcp@next`, `openwarrant-brief@next`) the seeder writes
that key, with the repository's visible test command (`arms.py` `DONE_TEST`,
`write_done_test`):

| repository | `done_test` |
|---|---|
| tally | `python3 -m pytest -q` |
| knightlm | `cargo test --workspace --locked -j 2` |

Each is a line of the project facts every arm carries, and the seeder refuses a command
that is not: the key hands `next` no command another arm was not told. It is written last,
after the seeder has closed the earlier items, so seeding itself runs no test. alpha.7 and
older arms get nothing: they refuse a `[tickets]` key they don't know (`unknown field
done_test`, checked on alpha.7 and on `next` at `d9b524f5c`).

This is part of what `next` is, not an advantage the harness gives it: any repository set
up by `war init` on that version gets the key, and a repository set up by alpha.7 does not
have the feature to get. The comparison of `next` with alpha.7 is a comparison of versions,
and this is one of the things that changed. What it can change in the measures: a `war
done` over a failing visible suite is refused, which can only lower false completions; and
the test run's output comes back to the agent, which costs tokens.

The switch is `ADOPTION_OW_DONE_TEST`, on unless set to `0`, and it reaches only `@next`
arms. `run.json` records it (`ow_done_test`: the switch, the arms that carry the key, each
repository's command), and each cell's `result.json` its own (`done_test`). A run whose
`next` does not read the key stops before its first cell (`feature_problems`), instead of
seeding a repository `war` can't read.

**A new arm, opencode only: `openwarrant-brief@next`.** It is `openwarrant@next` (the same
build, seed, `AGENTS.md`, `done_test`, prompts, model and turn cap) with one difference,
the session's opencode config: `"instructions": [".openwarrant/next.md"]`, opencode's list
of files to load at startup beside `AGENTS.md`. `.openwarrant/next.md` is a file `war` will
keep current: the brief for the next ready ticket, there on the first turn without a
command. The harness writes nothing into it. A brief cell whose seeded repository has no
such file stops (`seed_repo`), since without the file the arm is its twin under another
name; the arm is refused under Claude Code. It is shown beside its twin
(`openwarrant-brief@next` beside `openwarrant@next`: the amendment 6 table and the paired
ratio) and against `plain`, with no win rule. Edits under `.openwarrant/` are tracker
edits, not code edits, like `docs/tickets/` (`TRACKER_PATHS`); no earlier run has the
directory.

**More cells.** The opencode runs so far used six directed items, T8 to T10 and K7 to K9.
The backlogs hold sixteen more, each with hidden tests and a reference solution, each run
as a directed cell under Claude Code in `full-1` and the `rem-c*` runs:

- tally: T1 to T7, and T11 (T7 and T11 carry an open question, as T8 and T10 do).
- knightlm: K1 to K6, K10 and K11 (none carries an open question; K7 to K9 do).

Under opencode only T1 has run (`oc-smoke-*`, at 25 turns), so how DeepSeek does on the
other fifteen at 60 turns isn't known; the pass step may separate arms there where it
couldn't on six.

The widest directed set that fits about three hours a replicate at `--jobs 3` is all 22
items on four arms, `plain`, `openwarrant@a7`, `openwarrant@next` and
`openwarrant-brief@next`: 88 cells, 22 pairs a replicate for every pair
(`plans/oc-a8-directed.json`). A directed cell at 60 turns took 327 s on average over
`oc-v5-1` to `oc-v5-3` (225 s in `oc-v5-2`, 366 s in `oc-v5-3`), so 88 cells three at a time
is 2.7 to 3.0 hours, less on a quiet machine. The MCP arms would make it 132 cells and
4.0 to 4.5 hours, so they are left out; relay cells (about 41 minutes each) are left out
too. Until the product keeps the brief file, the same plan without the brief arm is
`plans/oc-a8-directed-nobrief.json`: 66 cells, 2.0 to 2.2 hours. Both at `--turns 60`,
arms interleaved per item, under `systemd-run --user`, after checking the endpoint is free
and the registry holds crates. A second replicate is the same plan under a new run id.

Pooling under this amendment: `next` from here carries `done_test`, which `oc-v5-2`'s and
`oc-v5-3`'s `next` did not, so runs under this amendment are pooled with each other, and
the 12 earlier pairs are shown beside them, not folded in, unless the owner directs
otherwise before the pool is run.

**Claude Code cells** (`plans/cc-a8-directed.json`; not run, they cost money and wait for
the owner). One set: the 22 directed items on `openwarrant@a7` and `openwarrant@next`, 44
cells, with the plugin and `war mcp` as each version ships them (alpha.7's plugin from its
tag, `next`'s from the commit it was built from; `agent_argv`). `claude-sonnet-5-5`, 25
turns (the cap of every earlier Claude Code run, so their prices apply), `--jobs 2`.

- Expected cost: USD 6.86. That is each item's mean price in the openwarrant arm of the
  earlier Claude Code runs (USD 0.08 to 0.25 a cell, 3.43 for the 22 items of one arm).
  The dearest directed cell of any arm in any run was USD 0.40. About 45 minutes.
- The cap: `--session-budget-usd 0.65 --budget-usd 28.60`. Every session at its cap is 44 ×
  0.65 = USD 28.60, so new spend stays under USD 30 even then. With
  `--session-budget-usd`, `--budget-usd` is hard (`Purse`): each cell's cap is held before
  it starts, and a cell that does not fit beside what is spent and held never starts. (Without
  the flag `--budget-usd` is as it was: checked after a cell ends, with USD 8 a session.)
  Claude Code checks its own cap between turns, so a session can pass 0.65 by its last
  turn; the USD 1.40 left under 30 is for that.
- A session cut by its cap would be the harness's doing, not the arm's: the cell is rerun
  once for every arm or excluded for every arm (amendment 5's rule), and the report says so.

Claude Code runs are not pooled with opencode runs (amendment 2).

**Checked, no model** (`run.py a8-check`, scratch roots only; fixture agents). 25 checks
passed and 5 were skipped on 2026-11-12, on alpha.7 and `next` at `d9b524f5c`. A skipped
check is owed, not passed:

- *The token step.* Twelve cells all cheaper: the new version wins. All dearer: it loses.
  An interval holding 1.0: a tie, where the median step named a winner. Cheaper everywhere
  with one false completion: it loses on false completions. Cheaper everywhere with two
  fewer passes: it loses on passes. The recorded runs give the figures in the table above.
- *Against plain.* Every OpenWarrant arm has a row; the criterion reads `next`'s, met at
  0.9 and not met at 1.1.
- *The hard budget.* A cell whose cap doesn't fit is refused, and fits again once a running
  cell settles for less than it held.
- *`done_test`.* Both commands are lines of the facts, and a command that isn't is refused.
  Only `@next` arms carry the key, and the switch turns it off. alpha.7's seed has no key.
  The control: on alpha.7, a fixture that adds a failing test to tally's visible suite and
  then records T8 done (`fixture-claim-broken`) is recorded done, a false completion. A run
  whose `next` lacks the key stops before its first cell; with the switch off it seeds as
  before. **Skipped until `war` has the key** (no build of main reads it yet): that
  `next`'s seed carries it and `war` reads it, that the reference solution is still
  recorded done, and the refusal, that `war done` over the failing suite is not recorded.
- *The brief arm.* Its opencode config is its twin's plus the one `instructions` entry; it
  uses `next`'s build; Claude Code refuses it; a brief cell stops when `war` wrote no brief
  file. **Skipped until `war` keeps the file:** that the file is in the seed. **Owed
  before the first brief cell:** one short session on the endpoint, not a study cell,
  showing opencode loads the file at startup, as amendment 6 proved `war mcp`.
- *The plans.* Each holds directed cells only, the same cells for every arm, and the
  Claude Code plan can't pass USD 30.

Before the first run under this amendment: `next` is rebuilt from a main that has both
features, its commit, version string and sha256 recorded as amendment 6 recorded them;
`run.py a8-check` is run again and its skipped checks must pass; and the seed parity of
amendments 5 and 6 is checked again, where `next` and alpha.7 should now differ in the
`AGENTS.md` block, the `done_test` key, and whatever `war init` writes for the brief.

The spec study's self-checks (`run.py spec-selfcheck`) share this code. Their sections that
run no spec cell were rerun on 2026-11-12 after these changes, and all 25 checks passed: the
customer, the gates, the registry, the turn caps and argv (the adoption argv still carries
USD 8 a session), and an adoption cell at 25 turns. The sections that run cells with fixture
authors and workers (binaries, isolation, the spec matrix, the control) were not rerun: the
machine's build slots were taken. They are owed before the spec study's next run.

### Setup note to amendment 8 (2026-11-13): checked on a `war` that has both features

Made with no run live and no model called. Main at `764736059` (#421) has `[tickets]
done_test` (and `done_test_timeout`) and `.openwarrant/next.md`. A release build of it,
`/mnt/4tb/tmp/openwarrant-adoption/tools/war-764736059/war`, reports `war 1.0.0-alpha.7
(unreleased, commit 764736059d1f)`, sha256
`98ecd5f1be71e7cfbf178b4ed369e9a623d90cb5cebe45aa5a6bf0165f286841`. It is the build the
checks below ran on, not the build the runs will measure: shared tickets and read-path
speedups are still landing, so `@next` in the measured runs will be a later main, recorded
when it is built as amendment 6 recorded its own, with `a8-check` run again on it.

**Where the harness did not match the product, and what changed** (`arms.py`):

- *`war init` records a test itself.* On this build a plain `war init` writes
  `done_test` with the test it finds: `python -m pytest` for tally (from `pyproject.toml`),
  `cargo test --locked` for knightlm. Amendment 8 assumed init wrote no `[tickets]` table.
  Left alone, the seeder would have closed the earlier items with that key in place, running
  the suite on the host once for each, and would then have added a second `done_test` line,
  which is not valid TOML. Now `setup_openwarrant` takes init's line out (`strip_done_test`;
  deleting the line is the product's way to turn it off), the earlier items are closed with
  no key, and `write_done_test` sets the key last.
- *The key is set through the product.* `write_done_test` runs `war init --done-test
  "<command>"`, which sets or replaces the key in a repository already set up, and checks it
  reads back, instead of appending text to `openwarrant.toml`.
- *Which command.* The seed carries the command of the project facts, as amendment 8 fixed
  it (`python3 -m pytest -q`; `cargo test --workspace --locked -j 2`), not the one init
  finds. They run the same suites; the facts' is the one every arm is told, and knightlm's
  keeps the `-j 2` the facts ask for on a shared machine. So `next`'s key is what `war init
  --done-test "<the project's test command>"` gives, one step past a plain `war init`.
- *The switch off now removes something.* With `ADOPTION_OW_DONE_TEST=0` init's own line is
  taken out and none is set: `next` without the feature, where before it was `next` as it
  came.

Nothing changed for the brief: `war` writes `.openwarrant/next.md` on create, start, done
and release, untracked (the clone's `.git/info/exclude` names it) and per worktree, so the
seeded repository of every `@next` arm holds it (for tally's T8 cell, 450 bytes: the three
ready tickets). Only `openwarrant-brief@next` loads it. A parallel cell's worktrees would
start without it until a `war` command runs in them; the plans hold directed cells only.

**`run.py a8-check` on this build:** 28 passed, 1 skipped, 0 failed. The four skips of
2026-11-12 that needed the product now pass: `next`'s seed carries the key and `war` reads
its tickets; the reference solution is recorded done (ticked at observed, the journal
naming `python3 -m pytest -q`); over a failing visible suite `war done` refuses ("was not
ticked at observed: default (`python3 -m pytest -q`) exited non-zero ... Nothing was
ticked"), where alpha.7 records the item done; and the brief file is in the seed. Two
checks were added: init's own key against the seed's one key, with no item ticked at
observed during seeding, and the switch off leaving no key. Still owed, as before: one
short session on the endpoint showing opencode loads the brief file at startup.

### Setup note to amendment 8 (2026-11-13): the `next` of the first run, and what was checked on it

Written and pushed before the run it describes.

**`next` is main at `919add881`** (`919add881cf9c280c4211b5c3313f0e2727c2453`; since
`764736059`: #422 shared tickets and the duplicate refusal, #424 field fixes, #425 the
read-path speedups, #426 and #428 the opt-in `docs/work` layout, #429 a hosted `war` no
longer asking for its working directory at startup). Release build,
`/mnt/4tb/tmp/openwarrant-adoption/tools/war-919add881/war`; it reports `war 1.0.0-alpha.7
(unreleased, commit 919add881cf9)`, sha256
`321a3cbc7311fdc3423bc3a6faef5b74df590468b2354b87599cb05a544ab87f`, plugin ref `919add881`.
Every `@next` arm of `oc-a8-1` uses it. The run waits for the full gate on that commit
(Actions run 43522176313) to conclude success. This note first named main at `d088e6cd6`
(sha256 `185ef12bbf3009794ca328e8148c54c42ce5fccbfb668122b61309b421667c3b`), whose full gate
(run 43510274570) failed on two plants; no cell ran on that build, #429 is the fix, and the
checks below were made on `d088e6cd6` and made again on `919add881`, with the same results.
The model check was made on `d088e6cd6` only: #429 changes neither the brief file nor
opencode's config.

**Checked on it, no model.** `run.py a8-check`: 28 passed, 1 skipped (the model check
below), 0 failed. No harness change was needed for this build:

- *The duplicate refusal.* `war create` now refuses a title that reads as an open ticket's.
  Both backlogs were seeded with every ticket open (the relay's seed) and with T1 to T7
  done: all 11 tickets were created each time, with no refusal and no `--force`. The seeder
  titles a ticket `<id>: <title>`, so every title holds a word no other has (`T8`, `K10`),
  which the rule reads as distinct work.
- *The default actor's announcement.* The seeder, the tracker reader and the fixtures all
  set `OPENWARRANT_ACTOR`, so `war` has no default actor to announce to them; and with no
  actor set, `war view tickets --json` still writes only its JSON envelope to stdout, which
  the tracker parses.
- *Seed parity*, alpha.7 against `next`, per-seeding ids named (tally's T8 cell, and both
  backlogs fully open). Every ticket document is the same. The differences are: the
  `AGENTS.md` block `war init` writes (three lines reworded); `[tickets] done_test` in
  `openwarrant.toml`; `.openwarrant/next.md`, untracked; two `docs/work` lines in
  `.gitattributes`; and, in the journals of tickets the seeder closed, two fields `next`
  adds to each `work.claimed` event (`coordination.branch`, `coordination.repo`). The same
  events are journalled, in the same numbers.

**opencode 2.2.1 restored.** The study's copy of the pinned agent
(`tools/opencode/2.2.1/opencode`) was gone from the scratch tools directory, and the
owner's install, which `prepare` copies from, has moved to 2.2.7. The pin holds: the
published package `@opencode/cli-linux-x64@2.2.1` was fetched from the npm registry, its
tarball checked against the registry's sha512 integrity, and its binary put back in place
(it reports `opencode v2.2.1`; sha256
`32e95ec126d416ab833094447cf590290e17dd028a68b122b94d3539795c0dae`). No hash of the earlier
copy was ever recorded, so that it is the same bytes rests on the version and the
registry, not on a comparison.

**The owed model check, done.** The endpoint was up on 2026-11-13 and its model list
reports no price for `deepseek-ai/DeepSeek-V4.1-Flash`, as before. Two sessions of one
model call each, in the study's sandbox on tally's T8 cell (not study cells, scratch
roots): asked, without tools, whether its startup instructions hold a passage beginning
"OpenWarrant:", `openwarrant-brief@next` quoted `.openwarrant/next.md` whole, naming the
file (`/work/repo/.openwarrant/next.md`) and its three ready tickets by id;
`openwarrant@next`, the same seed without the `instructions` entry, answered `NONE`.
Neither called a tool. opencode loads the file at startup, and only for the brief arm.

**The run, `oc-a8-1`:** replicate 1 of `plans/oc-a8-directed.json`, 88 directed cells
(`plain`, `openwarrant@a7`, `openwarrant@next`, `openwarrant-brief@next` on the 22 items),
opencode 2.2.1 with `lithosai/deepseek-ai/DeepSeek-V4.1-Flash`, `--turns 60 --jobs 3`,
`ADOPTION_OW_DONE_TEST` on, one `systemd-run --user` unit, run root in `/dev/shm`. Replicate
2 is not started with it: the owner decides after this one.

### Results of amendment 8: `oc-a8-1`

Run 2026-11-13, 18:17 to 19:52, on harness commit `893e31c3d`, after the full gate on
`919add881` concluded success (Actions run 43522176313): the 88 directed cells of
`plans/oc-a8-directed.json`, `next` at `919add881` with `done_test` on. No harness error:
every session's transcript holds model steps and every grade ran, so nothing was rerun or
excluded. One `plain` session (tally T4) exited 1 after 27 turns, the agent's own end, and
passed. From `run.py report oc-a8-1` and `run.py pool oc-a8-1`:

| arm | directed passed | false completions | turns per session | ktokens per session | median ktokens per cell | median turns before first edit |
|---|---|---|---|---|---|---|
| `plain` | 22 of 22 | 0 | 33.5 | 1,725.9 | 1,603.2 | 8.5 |
| `openwarrant@a7` | 22 of 22 | 0 | 37.1 | 1,413.7 | 1,366.5 | 8.5 |
| `openwarrant@next` | 22 of 22 | 0 | 35.7 | 1,379.0 | 1,338.1 | 8.0 |
| `openwarrant-brief@next` | 22 of 22 | 0 | 37.6 | 1,475.4 | 1,339.9 | 8.0 |

**`openwarrant@next` against `openwarrant@a7`, by the rule:** false completions 0 and 0;
passes 22 and 22, a tie; paired token ratio 0.965, −3.5%, 95% interval −12.3% to +6.2% over
22 pairs. The interval holds 1.0: **a tie, no win.** (Amendment 5's step, for comparison:
medians 1,338,098 against 1,366,523, −2.1%, also a tie.)

**Against plain** (paired token ratio, 22 pairs each, descriptive):

| arm | tokens, arm over plain |
|---|---|
| `openwarrant@next` | 0.786: −21.4%, interval −32.8% to −7.8% |
| `openwarrant@a7` | 0.815: −18.5%, interval −31.2% to −3.2% |
| `openwarrant-brief@next` | 0.833: −16.7%, interval −30.9% to −0.2% |

The owner's release criterion, `openwarrant@next`'s ratio against `plain` at or below 1.0:
0.786, met on these cells, and its whole interval is below 1.0.

**The brief arm beside its twin:** `openwarrant-brief@next` over `openwarrant@next` 1.060,
+6.0%, interval −10.3% to +24.1%; the same 22 passes, no false completion, 8.0 turns
before the first edit in both, 37.6 turns a session against 35.7. Nothing separates them
at this size. No brief session read `.openwarrant/next.md` with a tool, and every one
still ran `war start`.

**What `done_test` caught: nothing, in this replicate.** In the 44 `@next` cells no `war
done` was refused. The transcripts hold no refusal (`not ticked at observed`,
`ticket.check-failed`), and the ticket journals in the kept bundles agree: 292 default
runs journalled (150 and 142), every one `pass`, every tick at `observed`, where alpha.7's
148 ticks are plain claims. The agents ran the suite before `war done`, as the `AGENTS.md`
block tells them to, so the default test ran over a suite already green. With no false
completion in any arm, there was nothing for it to catch here.

**Passed but not recorded** (not false completions): four sessions reached opencode's last
step (59 turns) with the work passing and the ticket still open, their last message
naming `war done` as the step left: K4 on `openwarrant@next` and on
`openwarrant-brief@next`, T1 on `openwarrant-brief@next` and on `openwarrant@a7`.

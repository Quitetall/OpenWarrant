# Backlog format (`ow-adoption/backlog/v1`)

Each study repository has one tool-neutral backlog, `repos/<repo>/backlog.toml`.
The seeders (`seed/`) write the same content into every arm in that arm's own
form: a CLAUDE.md section, PLAN.md, Spec Kit `tasks.md`, Beads issues, or
OpenWarrant tickets. No arm gets text another arm lacks.

```text
repos/<repo>/
  backlog.toml          the items, in the format below
  source.toml           where the seed tree comes from (a local git repo + commit, or `seed/`)
  seed/                 (synthetic repos only) the starting tree, committed as the first commit
  hidden/<ID>/          acceptance tests for item <ID>; never copied into an agent's repository
  reference/<ID>.patch  a reference solution; applies with `git apply` on top of the seed
                        plus every earlier item's patch in `order`
```

## backlog.toml

```toml
schema = "ow-adoption/backlog/v1"
repo = "tally"
order = ["T1", "T2", "T3"]          # a topological order of every item; reference patches apply in it

[project]
# Facts every arm's CLAUDE.md carries verbatim: what the project is, layout, build and test commands.
facts = """..."""
# How the grader prepares the tree before running checks (e.g. a build). Optional.
prepare = ["cargo", "build", "--locked", "-q"]

[[item]]
id = "T1"
title = "Short imperative title"
summary = "One line."
description = """Markdown: the change and why."""
acceptance = [
  "Each criterion one sentence, observable, matching what hidden/T1 checks.",
]
notes = """Design decisions that live only in the backlog (formats, names, rounding rules).
An agent that never reads the item cannot satisfy them."""
depends_on = []                     # ids that must be done first
check = ["python3", "-m", "pytest", "-q", "{hidden}"]   # run with cwd = repository root
timeout_secs = 300
```

`{hidden}` expands to an absolute path outside the repository holding a copy of
`hidden/<ID>/`; `{repo}` to the repository root. A check passes on exit 0.

## Open questions (v2)

An item may carry one open question whose answer arrives later, through each arm's own
update channel, the way an owner's reply would:

```toml
[item.question]
asked = "Should `--depth` count the root as level 0 or level 1?"
answer = "Level 1: `Assets` alone is depth 1."
```

The item's text in every arm ends with `Open question: <asked> The answer will be added to
this item; follow it.` The harness then delivers `answer` (directed: right after seeding, as a
separate commit or record; relay: after session 1) as:

| arm | where the answer lands |
|---|---|
| plain | an `Answer:` line under the item in `CLAUDE.md` |
| plans | a line under the item in `PLAN.md` and a handoff-log entry |
| speckit | `spec.md`'s `## Clarifications` (`- Q: … → A: …`), as `/speckit-clarify` writes |
| beads | a comment on the issue (`bd comments add`) |
| openwarrant | a note on the ticket (`war note`) |
| jira | a comment on the issue |

The answer must contradict the natural default, so that an agent that never reads the update
fails the item's hidden tests, and the hidden tests must check it.

## Invariants the study relies on (checked by `run.py validate`)

1. On the seed alone, every item's check fails.
2. With the reference patches for `order[0..=i]` applied, item `order[i]`'s check
   passes, and so does every earlier item's.
3. Every acceptance criterion is something a check exercises, and every check
   exercises only what the item's text states (description, acceptance or notes).
   A check never depends on a detail the item does not give.
4. Items are sized so one session can finish one: a few files, under ~300 lines changed.
5. (v2) For an item with a question, its check fails on the reference solution built with
   the natural default instead of the answer (`reference/<ID>.default.patch`), and passes
   with `reference/<ID>.patch`.

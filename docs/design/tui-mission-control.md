# OpenWarrant Mission Control: the interactive terminal UI

Owner-directed design, 2026-11-04, corrected against the code 2026-11-06 (t-3c16c W5).
`docs/TUI.md` is what the app does today. This file is why it has that shape and what
it may not do. Successor to the viewer of `OW-WAR-0112`.

---

## 1. The problem

The first terminal viewer mapped the protocol's taxonomy onto 14 panes, with low-level
validation rungs competing with daily work for keys. It was structurally safe (clean
terminal restoration, no key handling) but passive, and its newest two panes showed
numbers nobody had recorded. A person in front of OpenWarrant needs two things:

```
[1] WATCH   where the work stands, who is on it, what they are doing
[2] ACT     create, dispatch, answer, sign
```

The rest stays one key away: **Records** (`3`) over either mode, **Repos** (`p`) from
anywhere.

## 2. Terminal rule

1. **No signing keys.** The TUI never touches a private key, never invokes `ssh-keygen`
   and never reads `SSH_AUTH_SOCK`. Every signature is a child `war sign … --ssh-sign`
   that owns the terminal, and the `ssh-add -c` dialog is the human act.
2. **No state of its own.** It consumes the canonical providers (`status::build`,
   `roadmap_cmd::view_with`, `work::Work`, `graph::build`, `estimate::schedule`,
   `ticket::scopes`, `ticket::live`, `watch::fingerprint`). It holds a cursor.
3. **Every action is a real, tested command.** A key handler returns the argv as a
   value, and the test suite and a plant parse each with the real CLI
   (`Cli::try_parse_from`). An action that does not work yet reads as unavailable.
4. **Unknown is shown as UNKNOWN**, never estimated into a real-looking number.
5. **Terminal integrity.** Entered and left through a `Drop` guard; a panic restores the
   alternate screen, cursor and echo.

## 3. Watch

**Top lines.** Counts only, each from a record:

- *Tokens*: what the records hold, known or `UNKNOWN`. Sources are `war evidence go`
  node tokens and runtime capture receipts (`docs/design/tui-usage.md`). No session
  usage is recorded today, so a repository that has never run `war evidence go` reads
  `UNKNOWN`. Dollars appear only from a dated, configured price table, labelled as an
  estimate. There is no burn velocity: nothing records one.
- *Fleet*: claims, lease state, worktrees, collisions.
- *Progress*: counts from `status::build` and `roadmap_cmd::view_with`, for example
  `29/145 resolved`. There is no percentage and no gauge. A roadmap phase's exit
  criteria are not a measure a percentage can be taken of, and Warrants are not equal.

**Work** lists open items in any encoding, with remaining checklist items. There is no
rendered DAG. The critical path from `estimate::schedule` is marked on the rows, and
waiting and blocked states come from `graph::build`. The whole graph remains `war plan
frontier --all`.

**Claims** come from the lock files: actor, age, lease left, workspace and declared
paths. There is no agent heartbeat. What a lock records is the mtime the holder's `war`
commands renew, and the app labels it `lock renewed`. Collisions come from
`ticket::scopes::conflict`, and an undeclared claim collides with nothing.

**Live** reads the focused agent's terminal through Herdr, or without it `git status`,
`git diff --stat` and the journal's newest events for the selected workspace. It shows
terminal text and diff statistics, not a file diff stream: Herdr has terminal output,
and no notion of an agent's diff.

**Questions** are the open intake questions the console's board lists, across Warrants.
They are not gathered from running agents, which file them through `war ask`.

**Notices** are W3's notices, with how far each got.

## 4. Act

**Create** goes through `war create`, from a title and checklist items. The drafter is
not deterministic: `war plan --draft` runs whatever `[plan] drafter_argv` configures, so
the option appears only when that is set, and is described as a configured process.

**Dispatch** picks a task from the ready list, then an executor from `[go]` (there is no
built-in model list), then a write scope, then a worktree, then claims with
`claim_scoped_at`, through `war start --worktree --print --paths`, and launches. Three
adapters:

1. Herdr: `tab.create`, then `agent.start` in the tab's root pane.
2. Detached `war start --worktree`, logging to a file. `claude`'s session needs a
   terminal, so it stops at once this way, and for such a harness the next adapter is
   offered first.
3. `war start --worktree` in this terminal, until the session ends.

An executor in `[go.executors]` runs under `war evidence go`, by its route, not for one
chosen task, and shows as unavailable here.

`claim_scoped_at` refuses a declared scope that overlaps another in the same checkout, by
name, and the refusal is what the person sees. Across worktrees an overlap is a merge
matter: it is allowed, and Watch shows it as a collision.

**Answer** is `war plan answer` for a stage's question and `war note` otherwise. Neither
pushes anything to a running agent: the agent reads an answer at its next `war` call or
`war start`, unless Herdr can deliver it (`agent.prompt`). The panel says which. There
is no multi-choice answer until a question record has an options field; `war plan
interview` is read-only and is shown as such.

**Sign**: `space` checks, `a` all, `s` signs one or a batch in one dialog. A check
belongs to an act, not a row, so a queue re-read cannot move it. `v` shows the request;
the diff and evidence of what is being signed are read here too.

## 5. Herdr is an optional adapter

Everything works without it. It is a local daemon with a Unix socket and a line protocol
of its own: one JSON object per line, `{"id", "method", "params"}` answered by
`{"id", "result"}` or `{"id", "error"}`. It is not JSON-RPC. Its fields are nullable,
and only ids and status are promised.

The socket is `HERDR_SOCKET_PATH`, else the session `HERDR_SESSION` names
(`<config>/sessions/<name>/herdr.sock`), else `<config>/herdr.sock`. `ping` decides
whether Herdr is there. Every call runs on a background thread, so a dead socket never
holds a key. Used: `ping`, `agent.list`, `agent.read`, `agent.prompt`, `tab.create`,
`agent.start`, `events.subscribe`.

## 6. Structure

A key handler runs nothing and returns an `Effect`. The event loop performs effects.
Background threads read the records into a finished `Snapshot` and send it over a
channel, so a corpus reload never stalls a keypress. Input stays `crossterm::event::poll`
with no async executor (OW-ADR-0014).

```
tui/mod.rs             run, terminal guard, event loop, keys, top-level draw
tui/model.rs           the cursor and the plain data of a Snapshot
tui/reader.rs          background threads and the one read
tui/herdr_adapter.rs   Herdr, and the working tree without it
tui/watch.rs           Watch
tui/act.rs             Act
tui/records.rs         Records and Repos
tui/argv.rs            every command line the app builds
```

## 7. Keys

No key differs from another by case alone. `Tab` flips the modes, `1` and `2` go direct,
`3` calls up Records, `p` calls up Repos. The table is in `docs/TUI.md`, and a test holds
it to the `?` overlay.

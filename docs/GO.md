# Running the work graph: `war evidence go` and `war start`

OpenWarrant keeps one dependency graph of the work and runs it. External
tools plug in as executors of single nodes; the graph, the order and the
decision that work is done stay here (OW-WAR-0148 M15).

## The graph

`war plan frontier --all` prints it (`oh.war/work-graph/v1` with `--json`).
It is built from `war plan model`:

| node | id | done when |
|---|---|---|
| an item of a light Warrant | `t-x/i-y` | its checklist line is ticked |
| a light Warrant | `t-x` | its last item is ticked (with no items, it is the work itself) |
| an agent stage of a directory Warrant | `NS-WAR-0001/STAGE-001` | a submission is recorded (`war plan frontier`) |
| a record other work implements or waits on | `REQ-pr1` | at least one item implements it, and every one that does is done |

An edge `a → b` reads "a waits on b": an item's `after`, a record atom's
`depends_on` line, a Warrant on its items, a stage on the stages of every
milestone its milestone `depends_on`, and an item on whatever the record it
`implements` depends on. A stage also waits until its earlier milestone is
complete (its obligations established): a submission alone never opens the
next milestone.

A node is **ready** when it is not done, an executor can run it, nobody
holds it (a claim whose lease ran out holds nothing), and everything it
waits on is done. A **cycle** (`graph.cycle`) is refused by name: nothing in
it can ever be ready, so `war evidence go` starts nothing while one exists.
A record nothing implements never reads done, and is reported
(`graph.record-unimplemented`).

## Estimates and order

`war plan estimate` learns how long work takes from the journal: for every
tick (`ticket.item_done`), the time since the last claim on that item or its
Warrant. A tick with no claim before it is no sample, and is counted. A
node's estimate is the median of the most specific group that has samples:
same type and a shared label, a shared label, same type, all; with none, the
prior (`[go] prior_secs`, 30 minutes). Each estimate says its group and its
sample count.

A node's remaining length is its estimate plus the longest remaining length
of what waits on it. The **critical path** starts at the open node with the
longest remaining length that waits on nothing open. Ready nodes start in
this order:

1. those with an effective due date, earliest first. A light Warrant may
   carry `due = "YYYY-MM-DD"` (`war edit <id> --due 2026-11-01`; `--due
   none` clears it); work a deadline waits on inherits it. A date never makes
   a node ready, so it never reorders what waits on what;
2. then the longest remaining length (the critical path first);
3. then priority, age and id.

`war evidence go --dry-run` prints that order and starts nothing.

## `war evidence go`

```sh
war evidence go                 # also `war go`
war evidence go --max-nodes 5
war evidence go --dry-run
war evidence go --retry t-3f2a/i-9c01
```

For each ready node, in order, up to `[go] max_parallel` at once:

1. **Claim** it through M11's shared claims, as `<actor>@go-<run>` (`--as`
   names another). Another run, in another worktree, or in another clone
   with `[claims] remote`, is refused the same node by name. A node another
   run has landed is marked under git's common directory
   (`openwarrant/go/landed/`), so a second run never runs it again even
   though its own checklist does not show the tick yet.
2. **Worktree**: `.openwarrant/worktrees/<node>` on branch `war-go/<node>`,
   from the integration branch (`[go] integration`, default
   `war-go/integration`, made at HEAD the first time). The worktree
   directory and the files a session writes for itself are added to the
   clone's `info/exclude`, so the checkout stays clean.
3. **Dispatch**: the packet (`oh.war/go-packet/v1`: the node, its Warrant's
   description and notes, what it waited on, the checks that decide its tick,
   the declared allowed acts, where to work, and the answer expected, with
   the attempt's bindings) goes to the executor. Its bindings are journalled
   first (`go.dispatched`).
4. **Answer**: a §51 Stage Submission. It passes exactly the refusals `war
   evidence submit` applies (`submission.malformed`,
   `submission.self-completion`, `submission.unknown-dispatch`,
   `submission.dispatch-mismatch`), whatever executor sent it. An answer asks
   to `verify` (check and land it), `continue`, `block`, `amend` or `cancel`;
   no answer can say the work is done.
5. **Land** (for `verify`), under a lock shared by every run of the clone:
   commit the worktree; merge the integration branch into it; run the node's
   tests and KPIs there; move the integration branch with a compare-and-set
   (`[go] land = "merge"`), or push the branch and open a pull request with
   `[go] pr_argv` (`land = "pr"`; dependents then start from their
   dependencies' branches).
6. **Tick** the node in the checkout the run started from, at the level it
   earned: `observed` when its tests ran and passed, `claimed` otherwise. A
   node whose minimum is above what a run can earn (`independent`,
   `signed`) is left for a person.

What does not land goes back to the frontier, open and unclaimed, with a
note on its Warrant saying what happened, and a `go.ended` event:

| outcome | what happened |
|---|---|
| `crashed` | the performer died or exited without an answer |
| `timeout` | it ran past `[go] node_timeout_secs`; its process group was killed |
| `conflict` | merging the integration branch into its branch conflicted (the files are named) |
| `check-failed` | a test or KPI failed, or could not run (UNKNOWN is not a pass) |
| `refused` | the answer failed a refusal above |
| `continue` | the performer asked for another round |
| `adapter-failed` | an external executor could not be reached, or answered outside the contract |

After `[go] max_attempts` (3) such endings in a row, or when the performer
asks to block, amend or cancel, the node is **set aside for a person**
(`go.blocked`): `war next` lists it, with the reason and the command that
puts it back, `war evidence go --retry <node>`.

The run stops when nothing more is ready, when `--max-nodes` or `[go]
max_total_secs` is reached, or on SIGINT/SIGTERM (what is running is killed
and released, never landed), and says which.

### Parallelism

Several nodes run at once only in worktrees: `[go] max_parallel` above 1
with `isolation = "shared"` is refused (`go.no-containment`). `war evidence
perform`, which runs in the shared tree, still refuses `[perform]
max_concurrent` above 1.

### Budgets

- **Time** per node (`node_timeout_secs`, default 3600) is enforced: the
  process group is killed, or an external executor's wait abandoned.
- **Tokens** per node (`node_tokens`) are enforced where the harness reports
  usage (Claude Code's `--output-format json` does; an answer may carry
  `"usage": {"tokens": N}`). An answer over budget is not landed
  (`go.token-budget`). Where nothing is reported the run says UNKNOWN
  (`go.tokens-unknown`), never zero.
- **Kinds**: `[go] unattended = ["bug", "chore"]` lists the Warrant types
  that run unattended (`untyped`, `stage`); anything else is left for `war
  start`.

## Executors

### Native

`[go] harness = "claude"` runs `claude -p --output-format json
--permission-mode acceptEdits` in the node's worktree with the brief (the
packet as Markdown) on stdin; the answer is the fenced JSON at the end of its
result. `harness = "generic"` runs `harness_argv` with the packet JSON on
stdin and reads the submission from stdout. Placeholders `{node}`,
`{worktree}`, `{packet}`, `{title}`, `{brief}` are filled; the environment
carries `WAR_GO_NODE`, `WAR_GO_PACKET`, `WAR_GO_ROOT`, `WAR_GO_WORKTREE`.
An agent stage of a directory Warrant is performed by `[perform]
performer_argv` with its Dispatch, under `war evidence perform`'s admissions.

### External

```toml
[go.route]
hq = "hq"            # a Warrant labelled hq runs on the hq executor

[go.executors.hq]
adapter = "agent-hq"

[go.executors.linear]
adapter = "linear"
dispatch_argv = ["linear-dispatch"]
poll_argv = ["linear-answer", "{ref}"]
```

An adapter is two argvs and a JSON contract:

- **dispatch** reads the node on stdin (`stdin = "packet"`, `"brief"` or
  `"beads"`) and prints a reference on its last line;
- **poll** (every `poll_secs`) prints `{"status": "pending"}` or
  `{"status": "submitted", "submission": {...}}`. Anything else, a status
  like `done` or `closed` included, is no answer (`go.adapter-answer`): an
  executor answers with a submission, and the refusals and checks decide.

A submission's `artifact_refs` may name `git:<ref>`; it is fetched from
`[go] remote` and merged into the node's branch before landing.

| adapter | stdin | default argvs |
|---|---|---|
| `agent-hq` | brief | `gh issue create --title {title} --body-file - --assignee @copilot`; `gh issue view {ref} --json comments --jq ...` (the last fenced `json war-submission` comment) |
| `linear` | packet | none: give both |
| `gastown` | the node as Beads issue JSONL (`war admin export beads`) | none: give both |
| `argv` | packet | none: give both |

## `war start <id>`

The twelfth daily verb: one node, interactively.

1. Claims the node, from its worktree (so the claim's journal line is on its
   branch), as the acting agent.
2. Makes `.openwarrant/worktrees/<node>` on `war-go/<node>` from HEAD (or
   `--base`), or resumes it. The Warrant has to be in that commit
   (`go.start-uncommitted` otherwise).
3. With `[go] allowed_acts = "projected"` (the default), writes the session's
   settings inside the worktree only: `.claude/settings.local.json` with the
   Warrant's declared tools, paths (`Edit(/src/**)`) and commands
   (`Bash(make test)`) as permissions, and `OPENWARRANT_ACTOR` set to the
   claim's holder; and `.openwarrant/session.json`, which the plugin's edit
   guard reads. A settings file it did not write is left alone. With
   `declared`, it writes no settings and shows them.
4. Runs `[go] start_argv` (Claude Code: `claude "<brief>"`) there, or prints
   the command and the brief with `--print`.

A light Warrant declares its allowed acts in an optional
`atoms/35-allowed.md`:

```markdown
# Allowed

## Tools
- Read

## Paths
- src/parser/**

## Commands
- `cargo test -p parser`
```

A directory Warrant's declared paths are its deliverables' files. Inside a
`war start` worktree whose Warrant declares paths, the plugin guard
(`.claude/hooks/guard-pins.sh`) turns back an edit outside them; a Warrant
that declares none restricts nothing.

Nothing either command does writes a user's or global settings file, signs,
or reaches a signing key: every git call it makes forces signing off.

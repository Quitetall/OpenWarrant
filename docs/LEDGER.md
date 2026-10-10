# The file ledger

Why each file changed, kept beside the code (OW-WAR-0148 M17, decision 24).
Git records what changed; the ledger records the reason, in a line per
change, curated and pruned so an agent can read every file's reason at once.

## Two forms

- **Atoms, committed.** One small Markdown file per source file, at
  `docs/ledger/<path>.md` (`[ledger] dir`):

  ```markdown
  # Ledger: `src/auth/redirect.rs`

  Why this file changed, oldest first (`war admin ledger record`). Older entries fold into the `earlier` line; git keeps the full history.

  - earlier: 2 change(s), 2026-09-01..2026-09-14; Warrants t-1a2b; first version | read the cookie domain from config
  - 2026-10-02 4f0c…(40 hex) t-9c8d (commit): the redirect read the session before the cookie
  - 2026-10-05 77aa…(40 hex) - (deterministic): Bump dependencies
  - 2026-10-09 - t-b95c (agent): handle the empty return URL
  ```

  An entry is `<date> <commit|-> <warrant|-> (<source>): <why>`. `-` is an
  uncommitted change (recorded at done) or a change that cites no Warrant.
  Sources: `commit`, `deterministic`, `agent`, `agent-trace`, `git-ai`.

- **`.openwarrant/ledger.jsonl`, regenerated, never committed.** `war admin
  compile` writes one JSON line per atom (`oh.war/ledger-file/v1`:
  `{schema, path, atom, earlier?, entries[]}`), sorted by path, so an agent
  reads the whole repository's reasons in one go. `war view prime` points to
  it. `.openwarrant/.gitignore` (written by compile) ignores it, and
  `war check` refuses a commit that tracks it (`ledger.committed`).

## Writing entries

```sh
war admin ledger record              # HEAD: an entry for each file it changed
war admin ledger record main..HEAD   # each commit in a range, oldest first
war admin ledger record --why "..."  # agent mode: the uncommitted changes, or HEAD
```

The why comes from `[ledger] mode`:

| mode | why |
|---|---|
| `commit` (default) | the commit's `Ledger:` trailer, else the first paragraph of its body; falling back to `deterministic` when the message has neither |
| `deterministic` | the title of the Warrant the commit cites (`Warrant: <id>`), else the commit subject |
| `agent` | the line the agent gives with `--why`; `war done` asks for it |

A trailer speaks for every file (`Ledger: tighten the redirect`) or for one
(`Ledger: src/auth/redirect.rs: read the cookie first`). The Warrant is the
commit's `Warrant:` trailer, or `--warrant <id>`. Recording a commit twice
adds nothing. The ledger never records its own atoms, a `generated/`
projection, or `.openwarrant/`.

## Pruning

An atom keeps the last `[ledger] keep` entries (default 3). Older ones fold
into the one `earlier` line: how many, their date range, the Warrants they
cited, and their reasons joined and cut at 400 characters. With
`[ledger] token_budget = N`, entries fold instead until the atom fits about N
tokens (four characters each), keeping at least one. Git keeps the full
history: `git log --follow -- <path>`.

`war admin ledger prune` refolds every atom, for when `keep` or the budget
changes.

```toml
[ledger]
dir = "docs/ledger"     # default
mode = "commit"         # commit | deterministic | agent
keep = 3                # entries kept per file
# token_budget = 200    # fold by size instead of count
```

## Agent Trace and git-ai

The ledger reads and writes the two formats other tools capture
attribution in, so what Cursor, Entire or git-ai recorded flows in, and what
the ledger knows flows out. It does not invent a third.

**Agent Trace** (Cursor's RFC v0.1.0, <https://agent-trace.dev>):

```sh
war admin ledger export --agent-trace --out traces.jsonl
war admin ledger import --agent-trace traces.jsonl
```

Export writes one trace record per commit: `vcs {type: git, revision}`, each
file with one conversation whose contributor is `unknown` (the ledger
records why, not who) and whose range covers the file at that revision, and
the entries under `metadata["dev.openwarrant"].ledger` (the spec's
reverse-domain metadata, section 7.2). Import gives back exactly those
entries; a record from another tool gives each file an entry naming the
contributor type, model and lines (`source: agent-trace`). The spec's
example version is `0.1.0` while its schema pattern admits two parts;
export writes `0.1.0`, import reads any `0.1`.

**git-ai notes** (Git AI Standard v3.0.0, `refs/notes/ai`):

```sh
war admin ledger export --git-ai     # a note per commit the ledger names
war admin ledger import --git-ai     # every note under refs/notes/ai
```

Export adds one session to each commit's note: `s_` plus 14 hex of
sha256(`openwarrant:ledger`), `agent_id {tool: openwarrant, id: ledger,
model: none}`, and the commit's entries as JSON in
`custom_attributes["dev.openwarrant.ledger"]`. It attests no lines, and a
note git-ai already wrote keeps its attestations and prompts. Import gives
back the ledger's own entries, and for any other note an entry per file with
AI-attested lines (`s_` or legacy keys), naming the tool, model and lines
(`source: git-ai`). Push the notes with `git push origin refs/notes/ai`.

## The score

The `ledger` dimension of the score (docs/SCORE.md) is the share of files
changed in the window that have an atom.

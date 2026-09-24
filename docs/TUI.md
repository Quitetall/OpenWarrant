# `war` — the app

`war` with no arguments, at a terminal, opens the app — from anywhere.
Inside a repository it opens that project; anywhere else it opens
**Projects**, every repository you use, and `Enter` opens one. It is a **rendering**
(SAS §76.6, OW-ADR-0019, OW-ADR-0020): it issues commands and holds no key.
Every act it starts runs the same `war sign --ssh-sign` a hand would run, as
a child that inherits your terminal, so the `ssh-add -c` dialog is still the
human act. Nothing under `crates/openwarrant-cli/src/tui/` names `ssh-keygen`
or `SSH_AUTH_SOCK`; the battery greps for both.

Not at a terminal — a script, a pipe, `war </dev/null` — `war` exits 2 with
`tui.no-tty` and names the commands a script wants instead: `war status
--json`, `war console --json`, `war next --json`, `war check --json`. `war
--json` with no subcommand refuses the same way. `war tui` is the same app
by name; `war console` stays as the line-based fallback (over ssh, inside the
battery).

Every pane reads the functions the CLI already answers with — `console::board`,
`next::run`, `status::build`, `frontier::run`, `check::run`, `doctor::run`,
`journal_cmd::load` — and renders. A pane that computed a fact itself would be
a second answer to a question the records already answer. A pane that cannot
answer says so ("not established", "UNREADABLE …"), never a blank.

The bottom line of every pane shows the exact command behind the highlighted
row. The status bar names the project and its path, the SAS revision in force
and the number of acts awaiting a signature. The tree is re-read on its own (the
`war watch` fingerprint, every second, debounced) so a signature given in
another terminal shows up without a keypress.

## Setup

Shown first whenever setup is incomplete. Started outside any repository,
the app opens Projects instead, and Setup for the current directory is one
keypress away (`1`); with `--root` naming a directory that is not yet a
repository, it opens here and the other panes render "not initialized". The row names the step; `Enter` runs `war
init`, the conversation, in your terminal, and the app resumes where the
tree says. Steps: name the program, say who signs, confirm the key asks you
(`ssh-add -c`), record the SAS, sign the SAS, prepare the first Warrant, sign
it.

## Help

Two things. **What next** (the default): first, which `war` this is — its
version and path — with a warning when `war` on PATH is a different version,
naming the install command (a command you copy runs PATH's `war`, not this
one). Then the Setup step if incomplete, then
`war next`'s actions with humans first — each signing row carrying the dry
run's verdict, `[would record]` or `[would refuse: <rule>]`, and the refused
ones after the recordable ones — then `war doctor`'s non-passes, then
`war check`'s errors — one row per rule with a count — each carrying its
remedy (§76.2). `Enter` on a signing row signs; `x` runs an *auto* remedy
(never a signing act); `v` shows the request behind a signing row.
**Documents** (`d` toggles, `[` and `]` step): first
`docs/generated/CURRENT.md`, the master document `war compile` writes
(OW-ADR-0022) — read it first: what is authoritative here now, current by
construction. Then this repository's own
`AGENTS.md`, `CONTEXT.md` and `CONTRIBUTING.md` when they exist, then
the shipped ones embedded at build time so help never drifts from what `war
init` ships — this file, `QUICKSTART.md`, `README.md`, `docs/DEFINITIONS.md`,
`docs/SKILLS.md`, and `AGENTS.md` as `war init` renders it for your
namespace.

## Queue

`war sign --list`, numbered. `space` checks a row, `a` all, `n` none; `s`
signs the checked acts — one checked is `war sign <target> --ssh-sign`, more
are one batch, `war sign --batch=<targets> --ssh-sign`, in one dialog
(docs/SIGNING.md); `Enter`
signs the highlighted row; `v` shows its request (`war sign <target>
--show`). Presets and reasons are asked by `war sign` itself, in the child.

## Questions

Open questions, blocking first, with the asker's recommendation. `Enter`
shows the question; the bottom line is the `war answer …` that answers it —
a human's act, run in a shell.

## Frontier

Every stage with its state (`open`, `claimed`, `done`, `blocked`), its
milestone and executor kind, and the milestones it waits on. The bottom
line is the `war perform` / `war run` that starts an open stage.

## Corpus

Every Warrant with its rung (`invalid`, `draft`, `ready_to_resolve`,
`would_satisfy`, `resolved`) and how many of §56.1's thirteen are unmet.
`Enter` shows the thirteen by name, the unestablished obligations and the
blocking unknowns — read from `status::build`, the same numbers `war status`
and `war resolve --dry-run` report.

## Obligations

Per obligation: disposition, verifier, and — when a verification is
inadmissible — the refusal's own words. `Enter` shows the statement, scope
and gate.

## Evidence

Gate runs grouped by class with verdict and reason, and the receipt each is
bound to. `x` records the gate again (`war evidence record <alias> --gate
<gate>`), an auto remedy.

## Journal

Every Warrant's journal events, newest first: time, Warrant, event kind,
actor. `/` filters — by a day (`2026-09-22`), a kind (`authorize`), an actor.
An unreadable journal is a red `UNREADABLE` row, not an empty pane.

## Roadmap

The program's phases in dependency order (`war roadmap`): each with its
members, whether it is achieved, and the work placed in it that no Warrant
carries yet. When the roadmap on disk is not an accepted revision, the first
row says so; `Enter` on it signs a proposed revision. `Enter` on a phase hands
the terminal to `war roadmap edit` — add, rename, re-exit, re-tier, reorder
or remove phases by keystroke — which writes the phases atom, proposes the
revision, and ends in one `war sign roadmap --ssh-sign` dialog. No file is
opened (OW-ADR-0023).

## Projects

Every repository you use (OW-WAR-0115). Nobody registers one: any `war`
command that opens a repository remembers it, in
`$XDG_CONFIG_HOME/openwarrant/projects.toml` (default `~/.config/…`), outside
every repository. The list stores where a project is and when you last used
it — never what it says. Each row is read from that project's own records
through the CLI's functions: its branch, SAS, acts awaiting a signature,
blocking questions, and Warrants resolved. `war projects` prints the same
rows; `--json` gives them as an envelope.

`p` opens this pane from anywhere; `Enter` opens a project — the whole app
switches to it, as if `war` had started there. `n` asks for a directory,
creates it, and runs `war init` there. A project whose directory no longer
holds a repository shows as `(missing)`, never silently dropped; `war
projects --forget <path>` takes it off the list, and `--add <path>` puts one
on without running a command in it. `OPENWARRANT_NO_PROJECTS=1` stops the
remembering (the conformance battery sets it). The pane is read when you open
it, not every second: `r` re-reads it.

## Keys

| key | does |
|---|---|
| `0-9`, `Tab`, `Shift-Tab` | switch pane (`0` is Roadmap) |
| `p` | Projects: every repository you use; `Enter` opens one, `n` starts one |
| `j` `k` | move (in a document: next/previous document) |
| `/` | filter this pane; `Esc` clears |
| `Enter` | act on the row: sign (queue, help), open the detail (others), run `war init` (setup) |
| `v` | show the request behind a signing row |
| `space` `a` `n` | queue: check row / all / none |
| `s` | queue: sign the checked acts — more than one is a batch, one dialog |
| `x` | run the row's auto remedy |
| `c` | show the commit message `war commit --write` would use |
| `d` `[` `]` | help: documents on/off, previous/next |
| `r` | re-read the tree now |
| `?` | this table |
| `q` / `Ctrl-C` | quit |

## The terminal comes back

Raw mode and the alternate screen are entered through one guard whose `Drop`
leaves them, and a panic hook leaves them before the panic prints. `war tui
--panic-after-setup` is the fixture the battery uses: it panics on purpose
and the plant asserts the alternate screen was left.

## What it is not

It signs nothing itself, answers nothing itself, records nothing itself. It
has no `--json`: a terminal application has no envelope, and pretending
otherwise would be a second projection contract. It is one file,
`crates/openwarrant-cli/src/tui/mod.rs`, because that is the file
OW-WAR-0112's signed declaration names; a later Warrant may split it.

# `war` — Mission Control

`war` with no arguments, at a terminal, opens the app — from anywhere. Inside a
repository it opens that repository; anywhere else it opens **Repos**, every
repository you use, and `Enter` opens one. It is a **rendering** (SAS §76.6,
OW-ADR-0019, OW-ADR-0020): it issues commands and holds no key. Every signature
it starts runs the same `war sign --ssh-sign` a hand would run, as a child that
inherits your terminal, so the `ssh-add -c` dialog is still the human act. Nothing
under `crates/openwarrant-cli/src/tui/` names `ssh-keygen` or `SSH_AUTH_SOCK`; the
battery greps for both.

Not at a terminal — a script, a pipe, `war </dev/null` — `war` exits 2 with
`tui.no-tty` and names the commands a script wants instead: `war status --json`,
`war view console --json`, `war view next --json`, `war check --json`. `war --json`
with no subcommand refuses the same way. `war view tui` is the same app by name;
`war view console` stays as the line-based fallback (over ssh, inside the battery).

There are two modes. **Watch** shows where the work stands and who is on it; **Act**
is where you start things. `Tab` flips between them, and `1` and `2` go direct. `3`
calls up **Records** over either mode, and `p` calls up **Repos** from anywhere.
Three rules hold everywhere:

- The app holds no signing key.
- It keeps no state of its own. Everything it shows is read from the records, by
  the same functions the commands use, and the cursor is all it remembers.
- Every action is a real command, and the bottom line shows the command behind
  the highlighted row. The test suite parses every command line the app builds
  with the real CLI.

What was not recorded says `UNKNOWN`. A source that could not be read says so at
the top of Work, never as an empty list. A list that has nothing says so ("not
established", "UNREADABLE …"), never a blank.

The records are re-read on their own: a background thread watches the fingerprint
of the Warrants, tickets, claims and notices, and a change that has settled is read
into a finished snapshot, so a signature given in another terminal shows up without
a keypress, and a reload never holds a key.

## Watch

The top lines are counts, each read from a record:

- **TOKENS**: tokens the records hold, or `UNKNOWN`. There are no dollars.
- **CLAIMS**: live claims, how many leases are live and expired.
- **COLLISIONS**: pairs of claims whose declared paths `ticket::scopes` will not
  call disjoint. Claims that declared nothing conflict with nothing.
- **WORKTREES**, **SIGN**: linked worktrees, and acts awaiting a signature.
- **WARRANTS**, **PHASES**, **ITEMS**: progress as counts, from `status::build`,
  `roadmap_cmd::view_with` and `work::Work`. Never a percentage: Warrants are not
  equal, and a percentage would be a number nobody recorded.
- **CRITICAL PATH**: its length in nodes and the estimate from `war plan schedule`.
  An estimate, and labelled so.

The panels, moved between with `h` and `l`:

- **Phases**: each roadmap phase, its done members, and `war plan roadmap`'s word
  for it. A member no record answers for is counted as `UNKNOWN`.
- **Work**: every open item in any encoding, from `work::Work`, with its remaining
  checklist items. A `*` is on the critical path.
- **Claims**: who holds what, for how long, the lease left, when the lock was
  renewed, the workspace and the declared paths. *Lock renewed* is the holder's last
  `war` command and nothing more: a silent agent can be working, and a renewed lock
  does not prove one is.
- **Live**: with Herdr, the focused agent's terminal; `[` and `]` step the agents.
  Without Herdr, `git status` and `git diff --stat` for the highlighted claim's
  workspace, and the journal's newest events.
- **Questions**: open questions, blocking first.
- **Notices**: W3's notices to running claims, and how far each got: `posted`,
  `told` (`war` printed its line to the holder), `seen` (`war start` showed the
  change).

Herdr is optional. Everything works without it, and the top line says which it is.

## Act

The panels, moved between with `h` and `l`. Sign is the one that opens.

- **Create**: unavailable in this build; run `war create`.
- **Dispatch**: what can start now, in scheduler order. Unavailable in this build;
  the bottom line is the command.
- **Answer**: open questions. Unavailable in this build; the bottom line is the
  command.
- **Sign**: see Queue.

## Queue

The Sign panel: `war sign --list`, numbered. `space` checks a row, `a` all, `n`
none; `s` signs the checked acts — one checked is `war sign <target> --ssh-sign`,
more are one batch, `war sign --batch=<targets> --ssh-sign`, in one dialog
(docs/SIGNING.md); `Enter` signs the highlighted row; `v` shows its request (`war
sign <target> --show`). Presets and reasons are asked by `war sign` itself, in the
child. A check belongs to the act, not the row, so a queue re-read behind your cursor
never moves it. The terminal comes back as soon as the child succeeds; a child that
failed leaves its output up until `Enter`.

## Questions

Open questions, blocking first, with the asker's recommendation, in Watch's drawer
and Act's Answer panel. `Enter` in Watch shows the question; the bottom line is the
`war plan answer …` that answers it — a human's act.

## Records

`3` calls Records up over either mode; `[` and `]` step its views, `Esc` goes back
to the mode underneath, and `3` returns to the view you last read. `t` opens it on
Tickets.

## Tickets

The ticket loop (docs/TICKETS.md), read-only. One row per ticket — its state (open,
in progress, done), progress (`2/5`), priority, title, and who holds a claim —
ordered as `war view tickets` orders them, and beneath each ticket one row per item
still to do, marked `ready` when `war view ready` would offer it. The bottom line is
the command: `war admin claim <ticket>/<item>` on a ready item, `war view show
<ticket>` otherwise. `Enter` shows the ticket as `war view show` prints it. The
view reads `ticket::board`, the same answer those commands give.

## Frontier

Every stage with its state (`open`, `claimed`, `done`, `blocked`), its milestone and
executor kind, and the milestones it waits on. The bottom line is the command that
starts an open stage.

## Corpus

Every Warrant with its rung (`invalid`, `draft`, `ready_to_resolve`, `would_satisfy`,
`resolved`) and how many of §56.1's thirteen are unmet. `Enter` shows the thirteen by
name, the unestablished obligations and the blocking unknowns — read from
`status::build`, the same numbers `war status` and `war sign resolve --dry-run` report.

## Obligations

Per obligation: disposition, verifier, and — when a verification is inadmissible —
the refusal's own words. `Enter` shows the statement, scope and gate.

## Evidence

Gate runs grouped by class with verdict and reason, and the receipt each is bound to.
`x` records the gate again (`war evidence record <alias> --gate <gate>`), an auto
remedy.

## Journal

Every Warrant's journal events, newest first: time, Warrant, event kind, actor. `/`
filters — by a day (`2026-09-22`), a kind (`authorize`), an actor. An unreadable
journal is a red `UNREADABLE` row, not an empty list.

## Roadmap

The program's phases in dependency order (`war plan roadmap`): each with its members,
whether it is achieved, and the work placed in it that no Warrant carries yet. When
the roadmap on disk is not an accepted revision, the first row says so; `Enter` on it
signs a proposed revision. `Enter` on a phase hands the terminal to `war plan roadmap
edit` — add, rename, re-exit, re-tier, reorder or remove phases by keystroke — which
writes the phases atom, proposes the revision, and ends in one `war sign roadmap
--ssh-sign` dialog. No file is opened (OW-ADR-0023). The same phases, with each
member's rung, are the Roadmap section of `docs/generated/CURRENT.md`; earlier
revisions are in `HISTORY.md`.

## Setup

Called up first whenever setup is incomplete and the repository has no open work.
Started outside any repository the app opens Repos instead, and setup for the current
directory is a row there; with `--root` naming a directory that is not yet a
repository, it opens here and the other lists say so. The row names the step; `Enter`
runs `war init`, the conversation, in your terminal, and the app resumes where the
tree says. Steps: name the program, say who signs, confirm the key asks you (`ssh-add
-c`), record the SAS, sign the SAS, prepare the first Warrant, sign it.

## Help

Two things. **What next** (the default): first, which `war` this is — its version and
path — with a warning when `war` on PATH is a different version, naming the install
command (a command you copy runs PATH's `war`, not this one). Then the Setup step if
incomplete, then `war view next`'s ready ticket items (shown, never run from here),
then its actions, an agent's workable ones first and the human acts after them (M9) —
each signing row carrying the dry run's verdict, `[would record]` or `[would refuse:
<rule>]`, and the refused ones after the recordable ones; with nothing ready, the
plain line `war view next` prints ("nothing tracked; work freely") — then `war admin
doctor`'s non-passes, then `war check`'s errors — one row per rule with a count — each
carrying its remedy (§76.2). `Enter` on a signing row signs; `x` runs an *auto* remedy
(never a signing act); `v` shows the request behind a signing row.

**Documents** (`d` toggles, `[` and `]` step): first `docs/generated/CURRENT.md`, the
master document `war compile` writes (OW-ADR-0022) — read it first: what is
authoritative here now, current by construction. Then this repository's own
`AGENTS.md`, `CONTEXT.md` and `CONTRIBUTING.md` when they exist, then the shipped ones
embedded at build time so help never drifts from what `war init` ships — this file,
`QUICKSTART.md`, `README.md`, `docs/DEFINITIONS.md`, `docs/SKILLS.md`, and `AGENTS.md`
as `war init` renders it for your namespace.

## Repos

Every repository you use (OW-WAR-0115). Nobody registers one: any `war` command that
opens a repository remembers it, in `$XDG_CONFIG_HOME/openwarrant/projects.toml`
(default `~/.config/…`), outside every repository. The list stores where a repository
is and when you last used it — never what it says. Each row is read from that
repository's own records through the CLI's functions: its branch, SAS, acts awaiting a
signature, blocking questions, and Warrants resolved. Scratch, gone and linked
worktree entries are folded into a count. `war admin projects` prints every row;
`--json` gives them as an envelope.

`p` calls it up from anywhere; `Enter` opens a repository — the whole app switches to
it, as if `war` had started there. `n` asks for a directory, creates it, and runs `war
init` there. A repository whose directory no longer holds one shows as `(missing)`,
never silently dropped; `war admin projects --forget <path>` takes it off the list, and
`--add <path>` puts one on without running a command in it.
`OPENWARRANT_NO_PROJECTS=1` stops the remembering (the conformance battery sets it).
The list is read when you open it, not every second: `r` re-reads it.

## Keys

| key | does |
|---|---|
| `Tab` | flip between Watch and Act |
| `1` | Watch: where the work stands, who is on it |
| `2` | Act: create, dispatch, answer, sign |
| `3` | Records over either mode |
| `p` | Repos from anywhere: `Enter` opens one, `n` starts one, `r` re-reads |
| `t` | Records, on Tickets |
| `Esc` | close Records or Repos; clear a filter |
| `h` `l` | move between panels |
| `j` `k` | move (in a document: next/previous document) |
| `/` | filter this list; `Esc` clears |
| `Enter` | open the row's detail; sign (Act, Help); run `war init` (Setup); open a repository |
| `v` | show the request behind a signing row |
| `space` `a` `n` | Sign panel: check row / all / none |
| `s` | Sign panel: sign the checked acts — more than one is a batch, one dialog |
| `x` | run the row's auto remedy |
| `c` | show the commit message `war admin commit --write` would use |
| `d` | Help: documents on/off |
| `[` `]` | Records: previous/next view; a document: previous/next; Watch: previous/next agent |
| `r` | re-read the records now |
| `?` | this table |
| `q` / `Ctrl-C` | quit |

No key differs from another by case alone.

## Where every old pane went

The app had 14 panes. Each function is listed here with where it lives now, so none
is lost.

| old pane | function | now |
|---|---|---|
| Observe | counts, fleet, leases | Watch, top lines |
| Observe | milestones and their tasks | Watch, Phases and Work |
| Observe | dependency tree, claims on it | Watch, Work and Claims |
| Observe | agent transcripts, `[` `]` | Watch, Live |
| Observe | questions | Watch, Questions |
| Orchestrate | draft and create | Act, Create |
| Orchestrate | dispatch | Act, Dispatch |
| Orchestrate | answer | Act, Answer |
| Orchestrate | batch sign | Act, Sign |
| Setup | the step, `Enter` runs `war init` | Records, Setup |
| Help | which binary, what next, remedies | Records, Help |
| Help | documents | Records, Help (`d`) |
| Queue | list, check, batch sign, sign row, show request | Act, Sign |
| Questions | list, detail, the answering command | Watch, Questions; Act, Answer |
| Frontier | stages and their commands | Records, Frontier |
| Corpus | rungs and the thirteen | Records, Corpus |
| Obligations | dispositions and verifiers | Records, Obligations |
| Evidence | gate runs; `x` records again | Records, Evidence |
| Journal | events, filter by day or kind | Records, Journal |
| Roadmap | phases; edit; sign a revision | Records, Roadmap; counts in Watch, Phases |
| Projects | every repository; open; new; re-read | Repos |
| Tickets | state, progress, items, detail | Records, Tickets; open items in Watch, Work |

## The terminal comes back

Raw mode and the alternate screen are entered through one guard whose `Drop` leaves
them, and a panic hook leaves them before the panic prints. `war view tui
--panic-after-setup` is the fixture the battery uses: it panics on purpose and the
plant asserts the alternate screen was left. A background reader that fails leaves the
app drawing and says so on the status line.

## What it is not

It signs nothing itself and records nothing itself. It has no `--json`: a terminal
application has no envelope, and pretending otherwise would be a second projection
contract.

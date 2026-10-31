# Tickets

**Tickets are Warrants.** A ticket is a Warrant in its light encoding: a
title is enough, and everything else is optional. `war view warrants` (also
`war view tickets`, `war view ls`) lists every Warrant, light, directory or read in
place, and `war view show <id>` shows any of them; docs/TYPES.md, "One Warrant,
three encodings", is the whole picture, with import and export for Beads,
OpenSpec and Spec Kit. This page keeps the word "ticket" for the light one.

A ticket is a piece of work written down so that anyone — you tomorrow, a
colleague, another agent — can pick it up and finish what is left. You create
one with a sentence, break it into a checklist, and tick items off as they are
done. No step needs a signature, a review or anyone's approval, and tickets
themselves are optional: ordinary coding needs none.

A ticket is the compatibility name for an ordinary Warrant. New Warrants use
one Markdown source and keep their ID as checklists, checks and requirements
are added. Existing manifest-and-atoms records remain supported. Optional
human sign-off supplies assurance; it does not define whether work exists.
`war plan promote` is retained for legacy directory-workflow interoperability.

## The loop in three commands

```bash
war create "Add password reset" --item "Reset endpoint" --item "Email template"
war start                                   # takes the next ready ticket; prints only what its work needs
war done --note "POST /reset, rate-limited" # closes the ticket you hold: every item, in one call
```

`war start` claims the ticket whole, so nobody else takes it, and prints a short human brief. With `--json`, it also returns the complete
required task-context packet, including exact rules and source revisions. Run again,
it prints the one you hold, so an interrupted session resumes there.
`war view prime` lists what is open, one line per ticket. The finer steps stay:
`war admin claim t-3f2a/i-9c01` takes one item and `war done t-3f2a/i-9c01` ticks it.

`war create` prints the ticket's id (`t-3f2a`). Items have ids too (`i-9c01`).
Any command takes an id or a unique prefix of one: `war admin claim i-9c`,
`war view show t-3f`. When an item id appears in two tickets, name it with its
ticket: `t-3f2a/i-9c01`.

More, when you need them:

| command | what it does |
|---|---|
| `war plan add <ticket> "text" [--after <item>]` | append an item; `--after` makes it wait on another item, a ticket (`t-...`) or another ticket's item (`t-.../i-...`) |
| `war plan note <ticket> "text"` | a dated note under the ticket's **Notes**: a decision, a dead end, a link |
| `war view show <ticket>` | the ticket as a person reads it, with who holds what |
| `war view warrants` (or `war view tickets`, `war view ls`) | every Warrant: the tickets first (open, in progress or done, and how far along), then the directory Warrants and those read in place, each with its type; filters below |
| `war plan edit <ticket> --type bug -l ui -p 1` | change a ticket's type, labels, epic or priority |
| `war plan edit <ticket> --due 2026-11-01` | a due date: a scheduler runs work toward it first, never before what it waits on (`--due none` clears it) |
| `war start <item>` | claim it and open it in a worktree of its own, with the Warrant's declared tools, paths and commands as the session's settings (docs/GO.md) |
| `war evidence go` | run every ready item, critical path first, each in its own worktree, landing on `war-go/integration` (docs/GO.md) |
| `war plan estimate` | how long each open item should take, learned from claim-to-done times in the journals, and the critical path |
| `war admin release <item>` | give a claim back without finishing |
| `war admin heartbeat [<item>]` | renew the lease on your claims (any `war` command does too) |
| `war admin claim <item> --steal` | take a claim older than the TTL whose lease is still live |
| `war create "..." --draft` | ask the configured drafter (`[plan] drafter_argv`) to propose the items |
| `war create "..." --draft --records` | the drafter proposes typed records too (requirements, constraints, decisions, outcomes), and the items implement them; see "From a sentence to records" |
| `war create --issue 12` | make the ticket from GitHub issue #12 (below) |
| `war create "..." --implements REQ-pr1` | one item per record, its text the record's first sentence (docs/TYPES.md) |
| `war plan promote <ticket>` | draft a Warrant from the ticket when someone wants sign-off |
| `war admin import beads <file.jsonl>` | bring Beads issues in as tickets; also `war admin import openspec <dir>`, `war admin import speckit <dir>` (docs/TYPES.md, "Bringing work in") |
| `war admin export beads` | every ticket as Beads issue JSONL, on stdout |
| `war plan add <ticket> --test "<cmd>"` | an optional part: a test, a KPI (`--kpi <name> --cmd ... --direction max\|min [--target N]`), a milestone (`--milestone "<text>" [--min observed]`); docs/TYPES.md, "Optional parts and the tick ladder" |
| `war done <item> --check` | run the item's tests and KPIs and tick at **observed** only when they pass; a plain `war done` ticks as **claimed** |
| `war evidence kpi run <ticket>` | run every KPI, journal each value, and say latest, best and target |
| `war evidence verify <item>` / `war sign <item> --ssh-sign` | an independent verdict, or a human's sign-off, of one item: the **independent** and **signed** levels |

Every command takes `--json` and answers with the same `oh.war/report/v1`
envelope as the rest of `war`. Each reads only the ticket files and the claims,
so each answers in milliseconds on a repository of any size.

## Work on disk

New work is `docs/tickets/t-xxxx.md`: metadata, description, checklist and
optional typed records in one document. Its append-only journal lives beside
it under `docs/tickets/t-xxxx/journal.jsonl`. A title is enough to start;
`war plan add` adds items or checks to that same source.

Legacy records keep their existing `manifest.toml`, intent and checklist atoms.
`war create --legacy` selects that older layout explicitly. The tool reads
both; no bulk migration or signed-history rewrite is required.

**The file is the state.** A ticked box is a done item; the order of the lines
is the order of the work; `after` is what an item waits on. Edit it by hand and
the tool follows: add a line (it gets an id the next time the tool writes the
checklist), reorder, reword, tick a box yourself. The tool only ever rewrites
the one line it changes, or appends one, and leaves every other byte alone.

A ticket is done when every item is ticked. A ticket with no items is itself
the one item: claim it whole and `war done` it.

## Types, labels and epics

A ticket may say what kind of work it is and carry labels:

```bash
war create "Login crashes on an empty password" --type bug -l auth -l backend
war plan edit t-3f2a --type feature --unlabel backend -l ui
```

The types are the ticket profile's: `profiles/ticket.toml` ships `task`,
`bug`, `feature`, `chore` and `epic` under `[fields] types`, and a type it does
not declare is refused (`ticket.type-unknown`). Labels are free words
(lowercase, `[a-z0-9_-]`) unless the profile closes the set:

```toml
[fields]
types = ["task", "bug", "feature", "chore", "epic"]
labels = ["auth", "backend", "ui", "docs"]
labels_closed = true          # any other label is refused: ticket.label-unknown
```

An **epic** is a ticket other tickets are part of:

```bash
war create "Password reset" --type epic                     # t-e9a1
war create "Reset endpoint" --part-of t-e9a1 -i "POST /reset"
war view show t-e9a1                                             # its tickets, and 0/1 done
```

`war view show <epic>` lists its tickets with each one's state and progress, and
the epic's progress over them; `war view tickets` marks it `[epic: 1/3 done]`. An
epic with no items of its own is worked through its tickets: `war view ready` never
offers it whole. `--part-of none` detaches a ticket; a cycle (an epic part of
its own ticket) is refused (`ticket.part-of-cycle`).

Each of these is one optional line in `manifest.toml` (`type = "bug"`,
`labels = [...]`, `part_of = "t-e9a1"`), absent when unset: a ticket written
before they existed reads, and is worked, byte for byte as it was. `war edit`
adds, replaces or removes exactly that line and journals the change
(`ticket.edited`). In `war plan model` the epic link is a `part_of` relation, as
each item is `part_of` its ticket and each `after` is a `depends_on`.

## Finding tickets: filters and search

```bash
war view tickets --type bug                  # exactly the bugs
war view tickets --label auth --label ui     # carrying both labels
war view tickets --state in_progress         # open, in_progress, done, or a declared state
war view tickets --state in_review           # a ticket or one of its items is in review
war view tickets --epic t-e9a1               # the tickets part of an epic
war view tickets --text "empty password"     # a phrase, anywhere
war view tickets --search "reset tok"        # words, each the start of a word, any order
```

Filters combine: a ticket is listed when every filter given admits it, in the
usual order. `--text` and `--search` read the title, the description, the notes
and every item with its done note, ignoring case. A filter that can match
nothing by construction is refused rather than answered empty: a state that is
neither fixed nor declared (`ticket.filter-state-unknown`), a type the profile
does not declare (`ticket.filter-type-unknown`), a label outside a closed set.

`in_review` is a declared state (`[[states]]` in the ticket profile): an agent
that has finished an item and wants a second pair of eyes says so with
`war plan state t-3f2a/i-9c01 in_review`. It holds while the item is claimed and
lapses when the item is done; it never stands in for done (docs/TYPES.md).

## From a GitHub issue, and back

`war create --issue 12` reads issue #12 once, through the read-only
`[intake] fetch_argv` that `war plan --issue` uses (a `fetch_argv` naming a
write such as `close` or `comment`, or `api`, is refused before anything
starts), and makes the ticket from it: the issue's title, its body as the
description, and the link (`issue = 12`, `issue_url`, the public URL with no
token in it) in the manifest. One issue makes one ticket; a second
`--issue 12` is refused (`ticket.issue-linked`). `--issue-file` reads
`gh issue view <n> --json number,title,body,url` output from a file instead.

Writing back is off unless you turn it on:

```toml
[intake]
fetch_argv = ["gh", "issue", "view", "{id}", "--json", "number,title,body,url"]

[intake.writeback]
comment_argv = ["gh", "issue", "comment", "{id}", "--body", "{body}"]
close_argv = ["gh", "issue", "close", "{id}", "--reason", "completed"]
timeout_secs = 30
```

When the last item of a ticket made from an issue is done, `war done` runs the
comment once — `{body}` is a summary of what was done: each item with who and
the note, then the ticket's notes — and then the close, once. Each is an argv,
never a shell string; `gh`'s own login holds any token, and `war` passes the
environment through and records none of it. Without `[intake.writeback]`,
finishing the ticket runs nothing and says the issue was not written to.

A write that fails does not undo anything: the ticket is done (its checklist
was written first) and stays done, the close is not attempted after a failed
comment, and the issue is reported **UNKNOWN** by name
(`ticket.issue-unknown`, exit 2): whether GitHub changed is not something
`war` can know. Both outcomes are journalled (`ticket.issue_writeback`); finish
what did not land by hand.

## A day with tickets

Morning, a person files work:

```bash
war create "Password reset" --type epic
war create --issue 12 --part-of t-e9a1 -i "Reproduce" -i "Guard the empty password"
war create "Reset page" --type feature -l ui --part-of t-e9a1 -i "Draw the form"
```

An agent arrives knowing nothing:

```bash
war start       # takes the next ready ticket and prints what its work needs
# ... the work ...
war plan note t-77c1 "root cause is the unchecked empty string in login.rs"
war done --note "repro: empty POST body"
```

A second agent, in parallel, runs `war start` too and gets the next ticket
(the one taken is no longer ready), and finishes it; with write-back configured, issue
#12 gets the summary and is closed. Late afternoon the person looks:

```bash
war view show t-e9a1                   # the epic: 1/2 done
war view tickets --state in_progress # what is moving
war view tickets --search "reset" # everything about reset
```

Nobody signed anything. The next agent's `war view prime` shows only what is left.

## Claims

`war claim` takes an item (or, with a ticket id, the whole ticket) so no other
agent works on it. A second agent's claim is refused and names who holds it and
since when. Of two agents claiming at the same instant, exactly one wins.
`war done` refuses an item nobody claimed, or someone else did, so two agents
never finish the same thing. Under the vibe preset (`war init --vibe`,
docs/PRESETS.md) a `war done` on an item nobody holds and nothing blocks
claims it first, journalled as a claim and then the tick; one someone else
holds is still refused.

Claims are lock files, never committed; every claim, release and steal is
also a line in the ticket's journal, which is. In a git checkout the locks
live under git's common directory (`git rev-parse --git-common-dir`, then
`openwarrant/claims/`), so every worktree of one clone shares one set: an
agent in one worktree is refused an item an agent in another holds. Outside
git, and before this was so, they lived in `.openwarrant/state/claims/`; a
claim found there is still honoured from every worktree, and its holder can
finish or release it. `[tickets] claims_dir` names one directory instead.

A claim is a lease. It carries `lease_until`, 30 minutes after it was taken
(`[tickets] claim_lease_minutes`), and the holder renews it with
`war admin heartbeat` and with every `war` command it runs, so an agent at work
keeps its claims without thinking about them. A renewal touches the lock
file's modification time and nothing else. When a lease runs out, the holder
probably stopped: `war view ready` offers the item again (`lease ran out`), and a
plain `war claim` takes it, journalled as `ticket.claim_reclaimed` with whom
it was taken from and when their lease ended. A claim with a live lease that
is older than two hours (`claim_ttl_minutes`, counted from when it was
taken, whatever its renewals) can still be taken with `war admin claim --steal`,
journalled with whom it was taken from.

### Claims across machines

Off by default. With

```toml
[claims]
remote = "origin"
```

a claim is also published to that git remote as the ref
`refs/openwarrant/claims/<ticket>--<item>` (a commit whose message is the
claim), pushed with `git push --atomic --force-with-lease`, so the remote is
the compare-and-set: of two machines claiming one item, exactly one push
lands, and the other is refused by name (`claimed on the remote origin by
...`) and keeps no lock. The lock on this machine is taken first, so the
worktrees of one clone settle among themselves before anything is pushed.
`war done` first checks the remote still gives the claim to you (a lease that
ran out there may have been taken from another machine), then deletes the
ref; `war admin release` deletes it if it is still yours. A claim whose lease ran
out on another machine is reclaimed across the remote, and journalled from
its holder. Local renewals are a file touch; the remote's copy of a lease is
renewed by `war admin heartbeat`, and by any ticket command once less than half of
it is left. When the remote cannot be reached the claim is refused
(`work.claim-remote-unreachable`, UNKNOWN: whether the claim would have
held is not something `war` can know) and nothing is claimed. `war view ready`
reads this machine's claims only; a claim held elsewhere is refused at
`war claim`. Nothing is signed: the commit is written with `--no-gpg-sign`
under a fixed `war` identity, and pushes skip hooks.

### Branches that merge

Two agents on two branches tick two adjacent items, each add an item and a
note: git's text merge reads two touching edits and stops. Ticket files keep
their layout; `.gitattributes` names a merge driver for them
(`docs/tickets/*/atoms/*.md merge=war-ticket`), and `war admin merge-ticket`
merges a checklist item by item (keyed by id) and an intent's appended notes
side by side, under one `## Notes` heading. One item changed two different
ways is not merged for you: the driver leaves git's conflict markers and
stops the merge (`ticket.merge-conflict`). Journals are append-only lines and
merge with git's built-in `merge=union`. `war init` writes both
`.gitattributes` lines and configures the driver for its clone; in a clone
made since, run `war admin merge-ticket --install` once. Without the driver git
merges these files as text, as before.

### Writes that say what they read

`war done`, `edit`, `note`, `add` and `release` take `--if-rev <revision>`
(`if_rev` over MCP): write only if the target is still what the caller read.
`war view show <ticket> --json` gives the revisions, the same digests `war plan model`
reports: `revision` for the ticket (its manifest's sha256; `edit` changes it)
and `items[].revision` for each item (its checklist line's; ticking or
rewording the item changes it). Pass the item's for an item, the ticket's for
the ticket. A stale one is refused, `work.stale-revision`, naming the
revision now, and nothing is written; read again and retry. On `done` and
`edit` the compare and the write are one step. Without `--if-rev` every
command behaves as it always has.

### What `war next` offers first

`war next` and `war view ready` give one order, and `war evidence go`
starts work in the same one (FORMAT-1.0 §11.5). The candidates are the
open items nothing they wait on holds back and no live claim holds (a claim
whose lease ran out holds nothing), and each Warrant with no items that
nobody holds; a Warrant `part_of` another counts as an item of it. They are
ordered by:

1. `due`, earliest first; an item inherits the earliest `due` of everything
   that waits on it, so the work a deadline depends on goes first. None
   sorts last.
2. the remaining path, longest first: the item's estimate plus the longest
   remaining path of what waits on it (the critical path). Estimates are
   learned from the journals (`war plan estimate`): the median claim-to-done
   time in whole seconds (the lower middle value of an even count), and
   `[next] prior_secs` (default 1800) with no history.
3. `priority`, lowest number first (default 2);
4. the Warrant's `created_at` (none last), then its id, then the item's
   position. An item has its Warrant's due, priority, created_at and labels.

The answer is a function of the files, the journals, the claims and the
time, so ten agents asking at once get the same list and each claims the
first it can; the claim lock makes exactly one win. `war next --at
<RFC 3339>` answers as of a given time (which leases have run out is judged
then). With `--json`, each row carries its effective `due` and, when there
is more than one row to order, its `remaining_secs`.

### Reopening an item

`war admin reopen <item> --reason "..."` unticks a done item: its box opens
and its done suffix goes, every other byte stays, and the journal records
why (`ticket.item_reopened`; `work.item_reopened` in a Format 1.0 ticket's
journal). It takes `--if-rev` like every write, an item someone else holds
is theirs to reopen (`work.claimed-by-other`), and an open item is refused
(`ticket.not-done`).

### Format 1.0 tickets

A ticket can also be one Format 1.0 document (FORMAT-1.0 §4.1):
`docs/tickets/<id>.md` or `docs/tickets/<id>/ticket.md`, its frontmatter
holding `alias`, `uuid`, `summary` and the ticket keys (`priority`, `type`,
`labels`, `due`, `part_of`, ...), its items the box lines of its units.
`war claim`, `war done`, `war add`, `war note`, `war admin release`, `war
admin reopen`, `war next` and `war view ready` work it exactly as they work
a directory ticket: the same claims, the same `--if-rev`, the same order.
A Format 1.0 ticket with no items is worked as itself: it is a `war next`
candidate, `war admin claim t-x` and `war done t-x` act on the document, no line
is added, and its done is a `work.item_done` naming the document (`war
admin reopen t-x` undoes it the same way). An item waits on what its
document `depends_on` as well as on its own `after`. Each item's revision is the sha256 of its line; the ticket's is the sha256
of its frontmatter and preamble, the revisions `war plan model` gives. Its
journal lies in `docs/tickets/<id>/journal.jsonl` and is written in the
Format 1.0 envelope (below). A command that writes what such a ticket does
not have — `war edit` (a manifest), `war plan promote`, `war plan add --test` (a
checks atom) — is refused by name (`ticket.format-document`): edit the
document itself.

### The journal's two envelopes

A Format 1.0 work document's journal holds `v: 2` events (FORMAT-1.0
§11.2): `{v, id, document_uuid, type, class, actor_ref, occurred_at,
payload, idempotency_key}`, each line RFC 8785 JSON, the payload an object,
the type a `work.*` name (`work.claimed`, `work.item_done`, ...), and the
key the sha256 of `<document_uuid> LF <type> LF <RFC 8785 payload>`. A
directory ticket's journal keeps the `v: 1` envelope and the `ticket.*`
names it always had, so each journal has one shape. A reader reads both, in
one journal if need be, and never rewrites a `v: 1` line; a `ticket.*` type
reads as the `work.*` event of the same suffix. One fact is recorded once
whichever envelope holds it: an event with the type and payload (compared
as RFC 8785 bytes) the journal already has is a replay from the same actor
and `work.idempotency-conflict` from another.

`war check` reads every ticket's journal: an unterminated last line is
`work.journal-torn`; a line that does not parse, lacks a member, has a `v`
other than 1 or 2 or a key that does not match it is
`work.journal-malformed`; a line with `v` above 2 is UNKNOWN,
`work.journal-newer`; and a committed line (in `HEAD`) missing, changed or
out of order is `work.journal-rewritten` (lines a union merge inserted
between them are fine). Of two events about one target that disagree (a
done and a reopening), the later `occurred_at` wins, the greater `id` on a
tie. A tick
whose level marker the journal does not back (an `[observed]` with no
passing run recorded) reads as the level that is backed, and `war check`
and `war show` say so with a warning, `work.tick-unbacked`.

### Rule ids renamed

Format 1.0 draft 3 names the tracker's refusals under `work.`. For one
release every message also names the id it had before, `(formerly ...)`,
so a script matching the old id still finds it:

| now | formerly |
|---|---|
| `work.stale-revision` | `warrant.stale-revision` |
| `work.claimed-by-other` | `ticket.claimed-by-other` |
| `work.claim-remote-unreachable` (now UNKNOWN, not an error) | `ticket.claim-remote-unreachable` |
| `work.journal-rewritten` | `journal.rewritten` |
| `work.journal-malformed` | `journal.malformed` |
| `work.journal-torn` | `journal.torn-tail` |
| `work.idempotency-conflict` | `journal.idempotency-conflict` |

A claim is a name for coordination. It proves nothing about who someone is and
authorizes nothing. Who is acting comes from `--as <name>`, else the
`OPENWARRANT_ACTOR` environment variable, else `[project] performer` in
`openwarrant.toml` (default `claude`). Give each parallel agent its own name.

## From a sentence to records

```bash
war create "Add password reset by email" --draft --records --area password-reset-email
```

The drafter answers with records as well as items: an outcome, the
requirements that serve it, the constraints and decisions on them. They are
checked exactly as hand-written records are (docs/TYPES.md, "From a sentence
to records"), then written to one record atom under
`docs/records/<area>/`, and the ticket is created with items that read
`… (implements REQ-pre1)`. `war plan impact REQ-pre1` then names the item a
change to that requirement reaches. A proposal with an undeclared type, a
relation to nothing, or an id the program already has is refused by rule,
and nothing is written. To read the proposal first, use
`war plan "<sentence>" --records --draft`, then `--reviewed --apply`.

## For agents

Start every session with `war start`, in the same command as your first look
at the code (`war start && ls`). It claims the next ready ticket (or prints the
one you already hold) and gives only what its work needs: the description, the
open items, every note, and what it depends on. Do the work, then close it in
the same command as your last test run: `make test && war done --note "..."`,
which closes it only if the tests pass. Run alone, both work the same; the
chain only saves the turns. The plugin's SessionStart hook puts the ticket you
hold (or the open list) in front of the agent before its first turn, and says
nothing when no ticket is open (`[tickets] session_start = false` turns it off). `war view prime` lists what
is open, one line per ticket, and `war view prime <id>` shows one in full.
Leave what the next agent needs to know with `war note`. Do not ask the human
to sign anything during this loop; nothing in it needs a signature.

Every tick says how it was earned. A plain `war done` is **claimed**: your
word. When the item (or its Warrant) has a test or a KPI, `war done --check`
runs them and ticks at **observed** only when they pass; `war show` lists
them, and an item whose minimum is observed says so in `war view prime`. A failed
check refuses the tick by name and leaves the claim yours: fix the work and
run it again.

Over MCP (`war admin mcp`, for a harness without a shell; one with a shell is
better served by the commands above) the same loop is `war_start` and `war_done`, with
`war_prime`, `war_ready`, `war_claim`, `war_create`, `war_add`, `war_note`,
`war_show`, `war_tickets` and `war_heartbeat` around it.
Each takes an optional `actor`; `war_done` also takes `check` (`--check`);
`war_create` also takes `type`, `labels` and
`part_of`, and `war_tickets` the filters above (`type`, `labels`, `state`,
`text`, `search`, `epic`). Finding the right ticket is
`war view tickets --search "<words>" --json`, not reading every directory.

## Configuration

All optional, in `openwarrant.toml`:

```toml
[tickets]
dir = "docs/tickets"                        # where tickets live
claims_dir = "/srv/war/claims"              # unset: shared by every worktree, under git's common dir
compact_after_days = 7                      # done tickets older than this are one line in `war view prime`
session_start = true                        # the plugin's SessionStart hook shows the agent its held Warrant, or the overview; false: nothing

[claims]
claim_lease_minutes = 30                    # a claim's lease, renewed by the holder's war commands
claim_ttl_minutes = 120                     # after this long since it was taken, a live claim may be stolen

[next]
prior_secs = 1800                           # an estimate with no history (`[go] prior_secs` is read when absent)
```

`claim_lease_minutes` and `claim_ttl_minutes` under `[tickets]`, where they
were before Format 1.0 draft 3, are still read when `[claims]` does not set
them.

```toml
[warrants]
hints = false                               # `war create` prints no one-line hint about tests
```

`[intake]` and `[intake.writeback]` (GitHub in and out) are above; types,
labels and the closed label set are the ticket profile's `[fields]`.

## How tickets relate to Warrants

A ticket is the **working form** of a delivery Warrant: `profiles/ticket.toml`
declares `form = "working"` with two roles, the intent and
`ticket.checklist` (docs/PROFILES.md). A working-form record lives outside
`docs/warrants/` and is never compiled, authorized, verified or resolved; a
Warrant manifest that names the `ticket` profile is refused. `war check`
validates a ticket's structure — a malformed checklist line
(`ticket.checklist-malformed`), a duplicate item id (`ticket.item-duplicate`),
a blocker that names nothing (`ticket.blocker-unknown`) or a cycle
(`ticket.blocker-cycle`) — and never reports a ticket for lacking a signature,
evidence or a verification.

`war plan promote <ticket>` is the way into the contract: it runs `war plan new` for a
delivery Warrant, carries the ticket's description and checklist into the new
intent, and records `promoted_to` on the ticket. From there the Warrant needs
every atom a delivery Warrant needs, and authorizing and resolving it are a
human's acts, exactly as before. The ticket stays workable meanwhile.

Working a ticket does not make recorded evidence stale. A gate with no
declared `inputs` is bound to the tree it ran over, and the files the loop
writes — each ticket's manifest, journal, intent and checklist, and the claim
locks — are outside that binding, so `war claim`, `done` and `note` after
`war evidence record` leave the receipts admissible. Anything else in a
ticket's directory stays bound. The reasoning, and the limit it states:
docs/RESOLVING.md, "Working a ticket does not move the tree".

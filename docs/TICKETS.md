# Tickets

A ticket is a piece of work written down so that anyone — you tomorrow, a
colleague, another agent — can pick it up and finish what is left. You create
one with a sentence, break it into a checklist, and tick items off as they are
done. No step needs a signature, a review or anyone's approval.

Sign-off is still there when you want it: `war promote` turns a ticket into a
Warrant, and the review and signing path in the rest of these docs applies from
that point. Most work never needs it.

## The loop in five commands

```bash
war create "Add password reset" --item "Reset endpoint" --item "Email template"
war ready                                   # what can start now
war claim t-3f2a/i-9c01                     # take one; nobody else will
war done t-3f2a/i-9c01 --note "POST /reset, rate-limited"
war prime                                   # what the next person reads first
```

`war create` prints the ticket's id (`t-3f2a`). Items have ids too (`i-9c01`).
Any command takes an id or a unique prefix of one: `war claim i-9c`,
`war show t-3f`. When an item id appears in two tickets, name it with its
ticket: `t-3f2a/i-9c01`.

More, when you need them:

| command | what it does |
|---|---|
| `war add <ticket> "text" [--after <item>]` | append an item; `--after` makes it wait on another item, a ticket (`t-...`) or another ticket's item (`t-.../i-...`) |
| `war note <ticket> "text"` | a dated note under the ticket's **Notes**: a decision, a dead end, a link |
| `war show <ticket>` | the ticket as a person reads it, with who holds what |
| `war tickets` (or `war ls`) | every ticket: open, in progress or done, and how far along; filters below |
| `war edit <ticket> --type bug -l ui -p 1` | change a ticket's type, labels, epic or priority |
| `war release <item>` | give a claim back without finishing |
| `war heartbeat [<item>]` | renew the lease on your claims (any `war` command does too) |
| `war claim <item> --steal` | take a claim older than the TTL whose lease is still live |
| `war create "..." --draft` | ask the configured drafter (`[plan] drafter_argv`) to propose the items |
| `war create "..." --draft --records` | the drafter proposes typed records too (requirements, constraints, decisions, outcomes), and the items implement them; see "From a sentence to records" |
| `war create --issue 12` | make the ticket from GitHub issue #12 (below) |
| `war create "..." --implements REQ-pr1` | one item per record, its text the record's first sentence (docs/TYPES.md) |
| `war promote <ticket>` | draft a Warrant from the ticket when someone wants sign-off |

Every command takes `--json` and answers with the same `oh.war/report/v1`
envelope as the rest of `war`. Each reads only the ticket files and the claims,
so each answers in milliseconds on a repository of any size.

## What a ticket is on disk

```
docs/tickets/t-3f2a/
  manifest.toml          id, title, priority, who created it and when; type,
                         labels, epic and issue link when it has them
  atoms/10-intent.md     the description: the sentence, context, decisions, notes
  atoms/15-checklist.md  the checklist
  journal.jsonl          what happened, one line per event (append-only)
```

The two Markdown files are the ticket. Open them in an editor or on GitHub:

```markdown
# Checklist

- [x] Reset endpoint (i-9c01) — done by claude, 2026-09-25: POST /reset, rate-limited
- [ ] Email template (i-4d11)
- [ ] Docs (i-77be, after i-4d11)
```

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
war edit t-3f2a --type feature --unlabel backend -l ui
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
war show t-e9a1                                             # its tickets, and 0/1 done
```

`war show <epic>` lists its tickets with each one's state and progress, and
the epic's progress over them; `war tickets` marks it `[epic: 1/3 done]`. An
epic with no items of its own is worked through its tickets: `war ready` never
offers it whole. `--part-of none` detaches a ticket; a cycle (an epic part of
its own ticket) is refused (`ticket.part-of-cycle`).

Each of these is one optional line in `manifest.toml` (`type = "bug"`,
`labels = [...]`, `part_of = "t-e9a1"`), absent when unset: a ticket written
before they existed reads, and is worked, byte for byte as it was. `war edit`
adds, replaces or removes exactly that line and journals the change
(`ticket.edited`). In `war model` the epic link is a `part_of` relation, as
each item is `part_of` its ticket and each `after` is a `depends_on`.

## Finding tickets: filters and search

```bash
war tickets --type bug                  # exactly the bugs
war tickets --label auth --label ui     # carrying both labels
war tickets --state in_progress         # open, in_progress, done, or a declared state
war tickets --state in_review           # a ticket or one of its items is in review
war tickets --epic t-e9a1               # the tickets part of an epic
war tickets --text "empty password"     # a phrase, anywhere
war tickets --search "reset tok"        # words, each the start of a word, any order
```

Filters combine: a ticket is listed when every filter given admits it, in the
usual order. `--text` and `--search` read the title, the description, the notes
and every item with its done note, ignoring case. A filter that can match
nothing by construction is refused rather than answered empty: a state that is
neither fixed nor declared (`ticket.filter-state-unknown`), a type the profile
does not declare (`ticket.filter-type-unknown`), a label outside a closed set.

`in_review` is a declared state (`[[states]]` in the ticket profile): an agent
that has finished an item and wants a second pair of eyes says so with
`war state t-3f2a/i-9c01 in_review`. It holds while the item is claimed and
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
war prime                         # open tickets, remaining items, claims, notes
war ready                         # what can start now
war claim t-77c1/i-1a2b
# ... the work ...
war done t-77c1/i-1a2b --note "repro: empty POST body"
war note t-77c1 "root cause is the unchecked empty string in login.rs"
```

A second agent, in parallel, takes what is left — `war ready` no longer lists
the claimed item — and finishes the ticket; with write-back configured, issue
#12 gets the summary and is closed. Late afternoon the person looks:

```bash
war show t-e9a1                   # the epic: 1/2 done
war tickets --state in_progress   # what is moving
war tickets --search "reset"      # everything about reset
```

Nobody signed anything. The next agent's `war prime` shows only what is left.

## Claims

`war claim` takes an item (or, with a ticket id, the whole ticket) so no other
agent works on it. A second agent's claim is refused and names who holds it and
since when. Of two agents claiming at the same instant, exactly one wins.
`war done` refuses an item nobody claimed, or someone else did, so two agents
never finish the same thing.

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
`war heartbeat` and with every `war` command it runs, so an agent at work
keeps its claims without thinking about them. A renewal touches the lock
file's modification time and nothing else. When a lease runs out, the holder
probably stopped: `war ready` offers the item again (`lease ran out`), and a
plain `war claim` takes it, journalled as `ticket.claim_reclaimed` with whom
it was taken from and when their lease ended. A claim with a live lease that
is older than two hours (`claim_ttl_minutes`, counted from when it was
taken, whatever its renewals) can still be taken with `war claim --steal`,
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
ref; `war release` deletes it if it is still yours. A claim whose lease ran
out on another machine is reclaimed across the remote, and journalled from
its holder. Local renewals are a file touch; the remote's copy of a lease is
renewed by `war heartbeat`, and by any ticket command once less than half of
it is left. When the remote cannot be reached the claim is refused
(`ticket.claim-remote-unreachable`) and nothing is claimed. `war ready`
reads this machine's claims only; a claim held elsewhere is refused at
`war claim`. Nothing is signed: the commit is written with `--no-gpg-sign`
under a fixed `war` identity, and pushes skip hooks.

### Writes that say what they read

`war done`, `edit`, `note`, `add` and `release` take `--if-rev <revision>`
(`if_rev` over MCP): write only if the target is still what the caller read.
`war show <ticket> --json` gives the revisions, the same digests `war model`
reports: `revision` for the ticket (its manifest's sha256; `edit` changes it)
and `items[].revision` for each item (its checklist line's; ticking or
rewording the item changes it). Pass the item's for an item, the ticket's for
the ticket. A stale one is refused, `warrant.stale-revision`, naming the
revision now, and nothing is written; read again and retry. On `done` and
`edit` the compare and the write are one step. Without `--if-rev` every
command behaves as it always has.

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
`… (implements REQ-pre1)`. `war impact REQ-pre1` then names the item a
change to that requirement reaches. A proposal with an undeclared type, a
relation to nothing, or an id the program already has is refused by rule,
and nothing is written. To read the proposal first, use
`war plan "<sentence>" --records --draft`, then `--reviewed --apply`.

## For agents

Start every session with `war prime`. It lists the open tickets with only their
remaining items, who holds which claim, the recent notes, and the done work
compacted to a line each once it is older than a week. Then `war ready`,
`war claim`, do the work, `war done --note`. Leave what the next agent needs to
know with `war note`. Do not ask the human to sign anything during this loop;
nothing in it needs a signature.

Over MCP (`war mcp`) the same loop is `war_prime`, `war_ready`, `war_claim`,
`war_done`, `war_create`, `war_add`, `war_note`, `war_show`, `war_tickets`
and `war_heartbeat`.
Each takes an optional `actor`; `war_create` also takes `type`, `labels` and
`part_of`, and `war_tickets` the filters above (`type`, `labels`, `state`,
`text`, `search`, `epic`). Finding the right ticket is
`war tickets --search "<words>" --json`, not reading every directory.

## Configuration

All optional, in `openwarrant.toml`:

```toml
[tickets]
dir = "docs/tickets"                        # where tickets live
claims_dir = "/srv/war/claims"              # unset: shared by every worktree, under git's common dir
claim_lease_minutes = 30                    # a claim's lease, renewed by the holder's war commands
claim_ttl_minutes = 120                     # after this a claim with a live lease may be stolen
compact_after_days = 7                      # done tickets older than this are one line in `war prime`
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

`war promote <ticket>` is the way into the contract: it runs `war new` for a
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

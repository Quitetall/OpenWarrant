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
done. Ordinary unverified completion needs no signature. Explicit Warrant and
repository requirements remain binding. Tracking itself is optional.

A ticket is the compatibility name for an ordinary Warrant. New Warrants use
one Markdown source and keep their ID as checklists, checks and requirements
are added. Existing manifest-and-atoms records remain supported. Optional
human sign-off supplies assurance; it does not define whether work exists.
`war plan promote` adds a field in place; `war plan new --from` is the legacy directory workflow.

## One item, three levels

There is one item and one format. Its level is derived from its fields and never
stored:

| Level | What makes it one | Done means |
|---|---|---|
| **Ticket** (default) | nothing extra | the agent says it is done |
| **Warrant** | a trigger, such as a test | the trigger passed |
| **Order** | a required human signature | the trigger passed, if any, and a person signed at the end |

`war create "..."` makes a Ticket. `--test "<command>"` makes it a Warrant, and
`war done` runs the test and closes the work only on a pass. `--sign` makes it an
Order: a milestone that ticks only when a person signs
(`war sign <id>/<item> --ssh-sign`), and `war view prime` marks it `Order`.
`war plan promote <id>` does either later, in place, with the same id. The
permissions `--path`, `--tool` and `--command` are separate from the level: they
say how the work may run, and none declared means no limit (docs/GO.md).

The daily commands are six: `init`, `create`, `start`, `done`, `status`, `check`.
Each takes `--interactive` for a person at a terminal (`--guided` on `init`), and
`war` alone opens Mission Control (docs/TUI.md). `war sign` stays for humans. The
earlier spellings in this page that are not among the six (`war admin claim`,
`war view show`, `war plan add`) are the groups' own; the top-level forms they
had (`war claim`, `war show`, `war add`) still work and are retiring
(docs/COMMANDS.md).

## The loop in three commands

```bash
war create "Add password reset" --item "Reset endpoint" --item "Email template"
war start                                   # takes the next ready ticket; prints only what its work needs
war done --note "POST /reset, rate-limited" # closes the ticket you hold: every item, in one call
war status                                  # where the work stands
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
| `war start <item> --worktree --print` | claim it and open it in a worktree of its own, with the Warrant's declared tools, paths and commands as the session's settings (docs/GO.md) |
| `war evidence go` | run every ready item, critical path first, each in its own worktree, landing on `war-go/integration` (docs/GO.md) |
| `war plan estimate` | how long each open item should take, learned from claim-to-done times in the journals, and the critical path |
| `war admin release <item>` | give a claim back without finishing |
| `war admin heartbeat [<item>]` | renew the lease on your claims (any `war` command does too) |
| `war admin claim <item> --steal` | legacy TTL takeover; managed writers must first be stopped and released |
| `war create "..." --draft` | ask the configured drafter (`[plan] drafter_argv`) to propose the items |
| `war create "..." --draft --records` | the drafter proposes typed records too (requirements, constraints, decisions, outcomes), and the items implement them; see "From a sentence to records" |
| `war create --issue 12` | make the ticket from GitHub issue #12 (below) |
| `war create "..." --implements REQ-pr1` | one item per record, its text the record's first sentence (docs/TYPES.md) |
| `war create "..." --test "<cmd>"` | a test from the start, repeatable: the same `## CHK-…` section `war plan add --test` writes. `war done` runs it and finishes the work only on a pass. Not with `--legacy` or `--records` |
| `war plan promote <ticket>` | add a signature (or with `--test`, a test) in place: same id, no new document |
| `war plan new --from <ticket>` | legacy directory-workflow adapter |
| `war admin import beads <file.jsonl>` | bring Beads issues in as tickets; also `war admin import openspec <dir>`, `war admin import speckit <dir>` (docs/TYPES.md, "Bringing work in") |
| `war admin export beads` | every ticket as Beads issue JSONL, on stdout |
| `war plan add <ticket> --test "<cmd>"` | an optional part: a test, a KPI (`--kpi <name> --cmd ... --direction max\|min [--target N]`), a milestone (`--milestone "<text>" [--min observed]`); docs/TYPES.md, "Optional parts and the tick ladder" |
| `war done <item> --check` | run the item's tests and KPIs and tick at **observed** only when they pass; a plain `war done` ticks as **claimed**, or at **observed** on `[tickets] done_test` passing ("For agents") |
| `war evidence kpi run <ticket>` | run every KPI, journal each value, and say latest, best and target |
| `war evidence verify <item>` / `war sign <item> --ssh-sign` | an independent verdict, or a human's sign-off, of one item: the **independent** and **signed** rungs of the tick ladder |

Every command takes `--json` and answers with the same `oh.war/report/v1`
envelope as the rest of `war`. Read-only views inspect the applicable records and claims; context compilation
and check execution perform their documented additional work.

## Work on disk

New work is `docs/tickets/t-xxxx.md`: metadata, description, checklist and
optional typed records in one document. Its append-only journal lives beside
it under `docs/tickets/t-xxxx/journal.jsonl`. A title is enough to start;
`war plan add` adds items or checks to that same source.

**The work layout (opt-in).** With `[tickets] layout = "work"` in
`openwarrant.toml`, `war create` writes a master instead,
`docs/work/w-xxxx/w-xxxx-<slug>.md` (`profile: ticket`), with its journal and
check output beside it in `docs/work/w-xxxx/`, where `war plan new` already
writes its work orders, by the same path: one id per clone, registered for
every worktree, and a title that reads as open work's is refused for both
(`--force` creates anyway). Every daily command reads either layout. It
becomes the default once the whole plant battery passes on it; until then
`war evidence go`, the evidence tree rule, context selection, interactive
create, the order signer and Mission Control are proven on `docs/tickets/`
only.

- **The id is identity.** `w-` and the hash of a fresh UUID. The slug is the
  title's, in the file name only, and cosmetic: rename the file and every
  command still finds the work by its id, and no digest moves. A second
  document under one id is refused (`format.document-duplicate`).
- **`t-` tickets stay valid** under either layout: `docs/tickets/t-xxxx.md` or
  a `docs/tickets/t-xxxx/` directory is read and worked where it is. Nothing
  moves by itself. `OPENWARRANT_TICKET_LAYOUT=work` (or `tickets`) sets the
  layout for one command, over the configuration.
- **The Roadmap.** `war admin compile` lists every open work order in
  `docs/ROADMAP.md`. A work order stands under a phase when its `part_of`
  names one (`part_of: roadmap/<phase>` in its frontmatter); any other, a
  child of an epic included, is listed under "Unphased". `war create` writes
  no view: the views are compiled on the default branch.

### Moving older tickets

`war admin migrate` moves the tickets kept outside `docs/work/` into it. It
is for a repository that chose the work layout; nothing requires it:

```sh
war admin migrate --dry-run     # every move, and every ticket that stays and why; nothing written
war admin migrate               # the same plan, with `git mv` so history follows
```

A `t-xxxx.md` (or `t-xxxx/ticket.md`) moves to `docs/work/t-xxxx/t-xxxx-<slug>.md`
with its bytes unchanged; a directory ticket is written as one document from
its manifest, intent and checklist, as `war create` writes one. The journal
and everything else in `docs/tickets/t-xxxx/` come along. **The id is kept**:
the store reads `docs/work/` by the frontmatter `alias`, whatever its prefix,
so `t-xxxx` is still `t-xxxx` to every command, claim, blocker and question.

A ticket is left in place, and named with the reason, when it has a recorded
act (`migrate.recorded-act`), a signed or verified tick (`migrate.signed`), a
file a resolved Warrant pins (`migrate.pinned`), a claim that still holds
(`migrate.claimed`), tests a directory ticket keeps in a checks atom
(`migrate.parts`), or cannot be read. The run is deterministic, a second run
moves nothing, and afterwards `war check` passes and the views list the same
work in the same states. Converting another tool's format needs a drafter and
a review, as `war plan` has; that path is not built.

A legacy directory delivery Warrant still uses its Dispatch, Submission and resolution
workflow. `war done` names that workflow instead of reporting an unknown ticket;
`war sign resolve --dry-run <alias>` lists its remaining requirements. Preparing a
legacy stage's worktree creates neither a claim nor a Dispatch.

Legacy records keep their existing `manifest.toml`, intent and checklist atoms.
`war create --legacy` selects that older layout explicitly. The tool reads
every layout; no migration or signed-history rewrite is required.

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

New Markdown sources use header fields. In legacy sources each is one optional
line in `manifest.toml` (`type = "bug"`,
`labels = [...]`, `part_of = "t-e9a1"`), absent when unset: a ticket written
before they existed reads, and is worked, byte for byte as it was. `war plan edit`
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

`war admin claim` takes an item (or, with a ticket id, the whole ticket) so no other
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

One work order, one place. A claim records where its work is done: the
repository (`[project] name`, and its `origin` URL without credentials), the
worktree, the branch and the commit it started from. `war start` prints it as
the brief's first line, once; Mission Control's claim detail shows the same
line, and the JSON of `war status <id>` (`places`) and `war status --next`
(`held`) carries it, while their text does not repeat it:

```
work in /src/wmd/.openwarrant/worktrees/t-1a2b (repo wmd, branch war-go/t-1a2b, from 1a2b3c4d5e6f)
```

A plain `war start` in a checkout records that checkout. While the lease is
live, its holder's `war start`, `war done` and `war note` on that work from
any other worktree are refused (`work.other-worktree`), naming the one to `cd`
to; another actor is refused by the holder's name (`work.claimed-by-other`), as
before; a read never is, and `war done --release` gives it back from anywhere. While you
hold live work in this checkout, claiming for another actor here (`war admin
claim <id> --as <other>`) dispatches it beside yours, and is refused
(`dispatch.here`) unless asked for with `--here`; `war start --worktree <id>
--as <other>` gives it a worktree of its own. `--as` alone only names who you
are. Work of another repository (`war start hpr:t-abc`, through
`[repos]`) is refused here (`work.other-repo`), naming that repository, the
pin this one reads it at, and the command that starts it there.

A claim is a lease. It carries `lease_until`, 30 minutes after it was taken
(`[tickets] claim_lease_minutes`), and the holder renews it with
`war admin heartbeat` and with every `war` command it runs, so an agent at work
keeps its claims without thinking about them. A renewal touches the lock
file's modification time and nothing else. When a lease runs out, the holder
probably stopped: `war view ready` offers the item again (`lease ran out`), and a
plain `war admin claim` takes it, journalled as `ticket.claim_reclaimed` with whom
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
held is not something `war` can know) and nothing is claimed. A push that runs
past its 30 seconds is also UNKNOWN, but it may have landed, and the refusal
says so: the local claim is given back, a claim that landed refuses other
agents until its lease runs out, and a retry from the same session takes it
back. Nothing is signed: the commit is written with `--no-gpg-sign`
under a fixed `war` identity, and pushes skip hooks.

Every view reads the remote's claims too: `war status`, `war view ready`,
`war view next`, `war view prime`, `war view tickets` and Mission Control
show a claim held on another machine with its holder, its workspace there,
the lease the remote holds for it, and `elsewhere` (Mission Control: `REMOTE`).
`war view ready` and `war start` never offer it, and `war start <it>` is
refused by name. The remote is read with one `git ls-remote` at most once per
`[claims] remote_ttl` seconds (default 30) for every worktree of the clone,
kept in `<git common dir>/openwarrant/remote-claims.json`; this clone's own
push reads it again at once. A remote that cannot be read is `UNKNOWN`, never
free: the views say so (`work.claims-remote-unknown`), `war status` prints
`Ready: ?`, and they exit as an UNKNOWN does. Mission Control reads it on its
reader thread, never on a keypress. Without `[claims] remote` nothing here asks
the network.

### Two machines

One repository, one git remote both machines can push to, nothing else to run.
Set it once and commit it:

```sh
war init --claims-remote origin     # writes [claims] remote = "origin" to openwarrant.toml
git commit -m "claims through origin" openwarrant.toml && git push origin HEAD
```

On the other machine, `git pull`. From then on:

```sh
# machine B
war start t-1a2b                    # claims it here and on origin
# machine A
war view ready                      # t-1a2b is not offered; `held elsewhere: t-1a2b by bob (on origin, ...)`
war start t-1a2b                    # refused (work.claimed-by-other): claimed on the remote `origin` by bob ...
```

A change pushed to the default branch from A reaches the claims on B it
concerns, once, as a notice on B's next `war` call after `remote_ttl` has
passed since B last looked: B fetches the branch into
`refs/openwarrant/upstream/<branch>` (no branch or remote-tracking ref of yours
moves) and compares what each of its claims watches. A claim it does not
concern hears nothing. A lease that runs out on B (its holder stopped renewing)
is taken on A by a plain `war start`.

### Shared work

Tickets are files in git, so a ticket made on one branch reaches another
only by a merge. Until then the clone announces it: `war create` registers
the new ticket under git's common directory
(`<common>/openwarrant/tickets/<id>`, beside the claims), with its title,
where it lives and its document.

- **Every worktree has it at once.** `war status` counts it, and `war view
  prime`, `ready`, `next`, `tickets` and Mission Control list it by title,
  marked `elsewhere: worktree <path> (branch <name>)`. Nothing is committed
  or fetched for that.
- **An id is one ticket's.** The register gives an id out once per clone, as
  a claim's lock is given. Two tickets under one id (two UUIDs, from a merge
  or another clone) are flagged `ticket.id-collision`, naming both; the one
  in this checkout stays listed.
- **`war start <id>` works it anywhere.** On a ticket held elsewhere it first
  writes the document into this worktree at the same path (`war done`,
  `war plan add` and `war plan note` do the same). Both branches then carry
  one ticket, and the merge driver below merges the two copies item by item.
- **Kept current, then retired.** Each `war done`, `war plan add` or
  `war plan note` refreshes the entry. It is dropped when the ticket is done,
  or once its file is on the default branch, where everyone has it through
  git.
- **Other clones.** With `[claims] remote` the entry is also a ref,
  `refs/openwarrant/tickets/<id>`: a commit whose message is the record and
  whose tree holds the document. Its first push expects no ref, so the remote
  settles which clone an id belongs to (`ticket.id-taken` for the loser, which
  creates again). Other clones list it as `elsewhere: remote <name>`, read at
  most once per `remote_ttl`. With no remote nothing asks the network. A
  remote that cannot be read is `UNKNOWN`
  (`work.claims-remote-unknown` names the tickets too, and `war status`
  prints `Ready: ?`), never an empty list. A create the remote could not be
  told of still makes the ticket, with a `ticket.unannounced` warning.

**No duplicates.** `war create` refuses a title that is, or reads as, an open
ticket's, wherever that ticket lives:

```
$ war create "Add export to CSV"
refused (ticket.duplicate): "Add export to CSV" is the same work as t-d5d7 ("Add CSV export"),
open in worktree /src/app-a (branch main) (docs/tickets/t-d5d7.md). `war start t-d5d7` works on
it; `--force` creates this one anyway
```

Titles are compared by their words: lowercased, without filler words (`the`,
`to`, `for`, ...) and without `-s`, `-ed`, `-ing` endings; a number belongs
to the word before it (`plant 45`). The same words in any order are the same
work. Otherwise two titles read as one when every word
of one (three or more) is in the other and covers at least 60% of it, so a
rewording that adds or drops words is caught. Two titles that each hold a word the other
lacks are distinct work, however much else they share: "Add CSV export" and
"Add JSON export" are both created. A synonym is therefore not caught
("Reject duplicates" beside "Refuse duplicates"). A done ticket never blocks,
tickets under two different epics are not compared, and `war create --force`
(MCP: `force: true`) creates anyway. Two agents creating one title at the
same instant leave one ticket: a short lock named by the title's words lets
one through, and the other is refused naming it.

`war plan new` registers its work order's id the same way. A work order
another worktree holds counts for the duplicate check but is not listed until
it is merged.

### Branches that merge

Two agents on two branches tick two adjacent items, each add an item and a
note: git's text merge reads two touching edits and stops. Ticket files keep
their layout; `.gitattributes` names a merge driver for them
(`docs/tickets/*.md merge=war-ticket`, and the same for `docs/work/`), and
`war admin merge-ticket`
merges a checklist item by item (keyed by id) and an intent's appended notes
side by side, under one `## Notes` heading. One item changed two different
ways is not merged for you: the driver leaves git's conflict markers and
stops the merge (`ticket.merge-conflict`). One ticket added on both branches
(`war start` on a ticket held elsewhere) has no common ancestor in git; what
the two copies still agree on stands in for it, so a tick on one side and a
new item on the other both survive. Journals are append-only lines and
merge with git's built-in `merge=union`. `war init` writes both
`.gitattributes` lines and configures the driver for its clone; in a clone
made since, run `war admin merge-ticket --install` once. Without the driver git
merges these files as text, as before.

### When something changes under you

`war start` keeps what a claim was given beside the claim, as
`given/<lock>.json` in the claims directory: the packet's `source_manifest`, the digest of each
file, and the digest of each file that reaches the work from outside the
packet, by a relation, by the ticket's text naming it, or by overlapping
declared `paths`. A relation or a ticket added after the claim started is
found by `war start` and `war plan impact`, not before.

A change counts when it is on the default branch (`origin/HEAD`, else `main`,
else `master`): in the working tree of a checkout on that branch, committed or
not, or at that branch's tip, seen from any other worktree. A side branch is a
proposal and tells nobody. It is looked for at every `war` write and in `war
check --changed`, comparing digests, so plain edits count once one of those runs.
A file is no news to a claim when the default branch has not moved it since the
claim's line left it, when the claim's workspace already holds those bytes, or
when it is the actor's own edit in its own workspace.

A notice is a file under `<git common dir>/openwarrant/notices/`, which every
worktree sees. Only the claim's actor, in the claim's workspace, hears it:

    notice: docs/plan.md changed on main; run `war start t-1a2b`.

That line comes once, on stderr, at the end of the next `war` call, or in `notes`
under `--json` and over MCP; several changes are one line. `war view prime` shows
it until it is seen. `war start` prints only the changes, with a bounded diff,
in place of the brief, and records them as seen.

`war done` holds a change to the item's level, and so a widening of the `paths`,
`x.tools` and `x.commands` it declared at `war start`:

| Level | A change it has not seen | A widening since `war start` |
|---|---|---|
| Ticket | warns and closes (`done.unseen`) | nothing |
| Warrant | refused until seen (`done.unseen`), then its trigger runs | closes, flagged by name and journalled (`done.widened`, `work.scope_widened`) |
| Order | as a Warrant | refused by name until a human signs a new revision (`done.order-widened`) |

A change seen but not in the workspace is refused on a Warrant or an Order
(`done.behind`): a trigger run there says nothing about the new state. Take the
change in, by merging or otherwise, and `war done` again. A change to what was
signed of an Order takes a new signature (`done.order-amended`); an Order nobody
has signed yet is signed at its end, over what it says then, so it may be amended
meanwhile. `war sign approve <id>` asks for that signature, and only a person can
give it. If the level or whether a change reached the claim cannot be read, a
Warrant or an Order answers `UNKNOWN` and stays open (`done.level-unknown`,
`done.seen-unknown`, and `OPENWARRANT_NO_NOTICES` is no way past it), and a Ticket
warns and closes. A Warrant or an Order claimed without `war start` has no record
to compare, and says so (`done.unrecorded`).

Otherwise a notice problem fails no command: a read ignores it, a write warns
(`notices.unavailable`). Notices are off in a hosted or recording run and under
`OPENWARRANT_NO_NOTICES`. They are coordination, and sign nothing.

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

### What `war status --next` offers first

`war status --next` and `war view ready` give one order, and `war evidence go`
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
first it can; the claim lock makes exactly one win. `war status --next --at
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
`war admin claim`, `war done`, `war plan add`, `war plan note`, `war admin release`, `war
admin reopen`, `war status --next` and `war view ready` work it exactly as they work
a directory ticket: the same claims, the same `--if-rev`, the same order.
A Format 1.0 ticket with no items is worked as itself: it is a `war status --next`
candidate, `war admin claim t-x` and `war done t-x` act on the document, no line
is added, and its done is a `work.item_done` naming the document (`war
admin reopen t-x` undoes it the same way). An item waits on what its
document `depends_on` as well as on its own `after`. Each item's revision is the sha256 of its line; the ticket's is the sha256
of its frontmatter and preamble, the revisions `war plan model` gives. Its
journal lies in `docs/tickets/<id>/journal.jsonl` and is written in the
Format 1.0 envelope (below). A command that writes what such a ticket does
not have — `war plan edit` (a manifest), `war plan promote`, `war plan add --test` (a
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
and `war view show` say so with a warning, `work.tick-unbacked`.

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
nothing when no ticket is open (`[tickets] session_start = false` turns it off). Its UserPromptSubmit hook
does the same for the ticket a prompt names (by id, a capital code such as `T8`, a double-quoted phrase or
its whole title), through `war view prime --prompt`, within 6,144 bytes and without claiming it: take it with
`war start <id>`. Several matches add one line and no brief, and a prompt naming nothing adds nothing. A
harness with no such hook can read a short list from `AGENTS.md`, which `war admin agents-md --block --ready 3`
writes; only that command refreshes it, so run it before agents start. `war view prime` lists what
is open, one line per ticket, and `war view prime <id>` shows one in full.
Leave what the next agent needs to know with `war plan note`. Do not ask the human
to sign anything during this loop; nothing in it needs a signature.

Every tick says how it was earned. A plain `war done` is **claimed**: your
word. When the item (or its Warrant) has a test or a KPI, `war done --check`
runs them and ticks at **observed** only when they pass; `war view show` lists
them, and an item whose minimum is observed says so in `war view prime`. A failed
check refuses the tick by name and leaves the claim yours: fix the work and
run it again.

**The repository's own test.** With `[tickets] done_test` set, a plain
`war done` runs that command on anything no check of the ticket's own applies
to, and says `ticked at observed: default passed`. A failure
(`ticket.check-failed`) or a run that could not finish (UNKNOWN,
`ticket.check-unknown`) ticks nothing, names the output file and leaves the
claim yours. `war init` records the test it finds (`cargo test`, `npm test`,
`go test ./...`, `python -m pytest`, `make test`; several are joined with
`&&`, so no suite goes unasked) and says so in one line;
`war init --done-test ["<command>"]` sets or replaces it in a repository
already set up. Delete the line and `war done` ticks as claimed again.

- It runs once per `war done`, whatever that call ticks: every item of a
  ticket closed in one call rests on the one run.
- A ticket's own check takes precedence. Where a `## CHK-…` section evaluates
  the item or the whole ticket (or a directory ticket's checks file names a
  test or KPI for it), that decides and the default does not run.
- A pass is reused, not rerun, only for the same ticket, the same command and
  the same bytes: HEAD, every tracked difference and every new file git does
  not ignore, the ticket's own ticks aside. Change anything and it runs
  again. The line then reads `default passed (not rerun: nothing changed
  since it passed)`. A failure is never reused.
- An Order still waits for its signer: the default decides the agent's tick,
  never a signature.
- An older `war` refuses a repository whose `[tickets]` has the key
  (`tickets.config`, an unknown field). Upgrade every `war` that reads the
  repository before setting it.

**What is next, without asking.** When `war done` finishes a ticket it ends
with `next: <id> <title> (`war start <id>`)`, or with nothing when nothing is
ready. `war status` names the first two ready tickets under its `Ready:` line,
and `war view prime --ready` prints only the ready rows. `war` also keeps
`.openwarrant/next.md` current in each worktree: the brief of the work the
acting agent holds, else the ready rows, within 6,144 bytes, rewritten on
`war create`, `start`, `done` and a release, and removed when nothing is held
or ready. It is never committed (the clone's `info/exclude` names it) and is
for a harness that loads instruction files before the first turn
(docs/INSTALL.md, "opencode"). One worktree has one file: with several actors
in one worktree it follows whoever acted last.

Over MCP (`war admin mcp`, for a harness without a shell; one with a shell is
better served by the commands above) the same loop is `war_start` and `war_done`, with
`war_create`, `war_note` and `war_status` around it (`war_status` takes
`next`, `ready` and `target`; `war_start` takes `claim`; `war_done` takes
`check`, `release` and `reopen`). `war admin mcp --all` adds `war_prime`,
`war_ready`, `war_claim`, `war_add`, `war_show`, `war_tickets` and
`war_heartbeat`. Each takes an optional `actor`;
`war_create` also takes `type`, `labels` and
`part_of`, and `war_tickets` the filters above (`type`, `labels`, `state`,
`text`, `search`, `epic`). Finding the right ticket is
`war view tickets --search "<words>" --json`, not reading every directory.

## Questions

An agent that needs a decision asks, instead of guessing or stopping:

```bash
war plan note t-4332 --ask "sqlite or files?" --blocking   # --recommend "files", --needs-human
war plan note t-4332 --answer Q-001 "files, one per item"
```

(`war note` is the same command; `war plan ask` and `war plan answer` still work.)
The question is `docs/questions/<id>/Q-nnn.toml`, beside nothing else; a directory
Warrant's stays under its `questions/` and is asked as `<alias>/<stage>`.

**Who is asked.** Asking names a party: the holder of the parent work (`part_of`),
else the actor that dispatched the session (`war start --worktree` sets
`OPENWARRANT_DISPATCHER`), else the work's creator, else anyone, and never the
asker. A question that needs a person is asked of a person. The party sees it at
the top of `war status`, in `war status --next` with the exact answer command, in
`war view prime`, and in Mission Control's Act view (`a` answers). An answer is
written where the question was asked, in another worktree of the same clone too,
and the asker hears it once: on its next `war` call, or in its `war start` brief.
With `[claims] remote`, both reach the other clones the way change notices do.
`war start` on work with an open blocking question says so in one line, and does
not wait.

**Who may answer** follows the ladder (OW-ADR-0034):

| The question is on | Answered by |
|---|---|
| a Ticket or a Warrant | any actor except the asker; the answer records who and their kind |
| an Order, a directory Warrant whose text is signed, or an intake question | a person |
| anything, asked with `--needs-human` | a person |

A level that cannot be read needs a person. The asker answering its own question
is refused `question.own`; an agent where a person is needed, `question.agent`.
With `docs/authority/roles.toml` the answerer (`--as`, else the acting name) must
hold an assignment, and its kind is the register's. Without one, a person is
whoever is at a terminal with a git identity and in no agent session; any other
caller is an agent. An answer informs the work: a Warrant's trigger still decides
when it is done, and an Order still needs its signature.

## Configuration

All optional, in `openwarrant.toml`:

```toml
[tickets]
dir = "docs/tickets"                        # where tickets live
layout = "tickets"                          # "work": `war create` writes masters in docs/work/ (opt-in)
claims_dir = "/srv/war/claims"              # unset: shared by every worktree, under git's common dir
compact_after_days = 7                      # done tickets older than this are one line in `war view prime`
session_start = true                        # the plugin's hooks show the agent its held Warrant, the overview, or the ticket a prompt names; false: nothing
done_test = "cargo test"                    # what `war done` runs on work with no check of its own; no line: nothing runs
done_test_timeout = 600                     # seconds that run may take before it reads UNKNOWN (default 600)

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

`war plan promote <ticket>` raises an item's level by adding a field, in place:
a sign-off item with `min: signed`, so it is an Order, or with `--test` a test,
so it is a Warrant. The id does not change and no document is written.
`war create --sign` and `--test` do the same when the work is made.

`war plan new --from <ticket>` is the way into the directory contract, which is
the one encoding with authorize and resolve: it runs `war plan new` for a
delivery Warrant, carries the ticket's description and checklist into the new
intent, and records `promoted_to` on the ticket. From there the Warrant needs
every atom a delivery Warrant needs, and authorizing and resolving it are a
human's acts, exactly as before. The ticket stays workable meanwhile.

Working a ticket does not make recorded evidence stale. A gate with no
declared `inputs` is bound to the tree it ran over, and the files the loop
writes — each ticket's manifest, journal, intent and checklist, and the claim
locks — are outside that binding, so `war admin claim`, `done` and `note` after
`war evidence record` leave the receipts admissible. Anything else in a
ticket's directory stays bound. The reasoning, and the limit it states:
docs/RESOLVING.md, "Working a ticket does not move the tree".

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
| `war tickets` (or `war ls`) | every ticket: open, in progress or done, and how far along |
| `war release <item>` | give a claim back without finishing |
| `war claim <item> --steal` | take a claim whose holder went quiet (older than the TTL) |
| `war create "..." --draft` | ask the configured drafter (`[plan] drafter_argv`) to propose the items |
| `war promote <ticket>` | draft a Warrant from the ticket when someone wants sign-off |

Every command takes `--json` and answers with the same `oh.war/report/v1`
envelope as the rest of `war`. Each reads only the ticket files and the claims,
so each answers in milliseconds on a repository of any size.

## What a ticket is on disk

```
docs/tickets/t-3f2a/
  manifest.toml          id, title, priority, who created it and when
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

## Claims

`war claim` takes an item (or, with a ticket id, the whole ticket) so no other
agent works on it. A second agent's claim is refused and names who holds it and
since when. Of two agents claiming at the same instant, exactly one wins.
`war done` refuses an item nobody claimed, or someone else did, so two agents
never finish the same thing.

Claims are lock files in `.openwarrant/state/claims/`, which is never
committed; every claim, release and steal is also a line in the ticket's
journal, which is. A claim older than two hours is stale and
`war claim --steal` may take it; the steal is journalled with whom it was taken
from.

A claim is a name for coordination. It proves nothing about who someone is and
authorizes nothing. Who is acting comes from `--as <name>`, else the
`OPENWARRANT_ACTOR` environment variable, else `[project] performer` in
`openwarrant.toml` (default `claude`). Give each parallel agent its own name.

## For agents

Start every session with `war prime`. It lists the open tickets with only their
remaining items, who holds which claim, the recent notes, and the done work
compacted to a line each once it is older than a week. Then `war ready`,
`war claim`, do the work, `war done --note`. Leave what the next agent needs to
know with `war note`. Do not ask the human to sign anything during this loop;
nothing in it needs a signature.

Over MCP (`war mcp`) the same loop is `war_prime`, `war_ready`, `war_claim`,
`war_done`, `war_create`, `war_add`, `war_note`, `war_show` and `war_tickets`.
Each takes an optional `actor`.

## Configuration

All optional, in `openwarrant.toml`:

```toml
[tickets]
dir = "docs/tickets"                        # where tickets live
claims_dir = ".openwarrant/state/claims"    # point at a shared path to see claims across worktrees
claim_ttl_minutes = 120                     # after this a claim may be stolen
compact_after_days = 7                      # done tickets older than this are one line in `war prime`
```

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

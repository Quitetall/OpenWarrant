# Tickets

A ticket is a Warrant in its lightest form: work written down as a checklist
so anyone can pick it up. It is optional: ordinary coding needs no ticket.

```bash
war view prime                                 # open tickets, remaining items, claims, recent notes
war next                                       # what can start now
war claim t-3f2a/i-9c01                        # take an item; a second claim on it is refused by name
# the work
war done t-3f2a/i-9c01 --note "what you did"   # ticks the box and gives the claim back
war note t-3f2a "a decision, a dead end"       # context for the next agent
war create "Add password reset" --item "Reset endpoint" --item "Email template"
war add t-3f2a "Docs" --after i-9c01           # extend a ticket; --after makes it wait
```

No step needs a signature or anyone's approval. The file is the state:
`docs/tickets/<id>/atoms/15-checklist.md` is a Markdown checklist, and a hand
edit is read as written. Name yourself with `--as <name>` (or
`OPENWARRANT_ACTOR`) when several agents share the repository.

Finding work: `war view warrants --search "<words>"`, `war view warrants --state
in_progress`, `war view warrants --type bug`, `war show <id>` (`war view tickets` is the
older name of `war view warrants`). The list also holds the directory Warrants and
any OpenSpec or Spec Kit folder read in place; `war admin import` brings Beads,
OpenSpec or Spec Kit work in as tickets.

Over MCP the loop is `war_prime`, `war_ready`, `war_claim`, `war_done`,
`war_create`, `war_add`, `war_note`, `war_show` and `war_tickets`
([MCP](mcp.md)).

Sign-off is opt-in: a person who wants it runs `war plan promote <ticket>`, which
drafts a Warrant from the ticket. The ticket stays workable meanwhile.

# Tickets

A ticket is work written down as a checklist so anyone can pick it up. It is
optional: ordinary coding needs no ticket.

```bash
war prime                                      # open tickets, remaining items, claims, recent notes
war ready                                      # what can start now
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

Finding work: `war tickets --search "<words>"`, `war tickets --state
in_progress`, `war tickets --type bug`, `war show <ticket>`.

Over MCP the loop is `war_prime`, `war_ready`, `war_claim`, `war_done`,
`war_create`, `war_add`, `war_note`, `war_show` and `war_tickets`
([MCP](mcp.md)).

Sign-off is opt-in: a person who wants it runs `war promote <ticket>`, which
drafts a Warrant from the ticket. The ticket stays workable meanwhile.

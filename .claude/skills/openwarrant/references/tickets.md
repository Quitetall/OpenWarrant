# Tickets

A ticket is a Warrant in its lightest form: work written down as a checklist
so anyone can pick it up. It is optional: ordinary coding needs no ticket.

```bash
war start && ls                                # take the next ready ticket with your first look at the code
# the work
make test && war done --note "what you did"    # close it with your last test run: every item, in one call
war view prime                                 # what is open, one line each; `war view prime t-3f2a` for one
war note t-3f2a "a decision, a dead end"       # context for the next agent
war create "Add password reset" --item "Reset endpoint" --item "Email template"
war add t-3f2a "Docs" --after i-9c01           # extend a ticket; --after makes it wait
```

`war start` claims the ticket whole, so a second agent gets the next one
instead; run again, it prints the one you hold. Chained onto your first look
and your last test run, the two cost no turns of their own (and `war done`
runs only if the tests pass); run alone they work the same. With
`[tickets] session_start = true`, the plugin's session hook shows the ticket
you hold, or the open list, before your first turn. `war start t-3f2a` takes that one, and
`war done t-3f2a/i-9c01` ticks a single item.

No step needs a signature or anyone's approval. The file is the state:
`docs/tickets/<id>/atoms/15-checklist.md` is a Markdown checklist, and a hand
edit is read as written. Name yourself with `--as <name>` (or
`OPENWARRANT_ACTOR`) when several agents share the repository.

Finding work: `war view warrants --search "<words>"`, `war view warrants --state
in_progress`, `war view warrants --type bug`, `war show <id>` (`war view tickets` is the
older name of `war view warrants`). The list also holds the directory Warrants and
any OpenSpec or Spec Kit folder read in place; `war admin import` brings Beads,
OpenSpec or Spec Kit work in as tickets.

Over MCP the loop is `war_start` and `war_done`; `war_prime`, `war_create`,
`war_add`, `war_note`, `war_show` and `war_tickets` around it ([MCP](mcp.md)).

Sign-off is opt-in: a person who wants it runs `war plan promote <ticket>`, which
drafts a Warrant from the ticket. The ticket stays workable meanwhile.

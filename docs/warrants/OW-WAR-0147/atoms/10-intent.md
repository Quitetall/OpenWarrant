---
schema: oh.war/atom/v1
warrant_uuid: 01a0da65-9e85-7531-a20e-8e695ecfa9ee
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The owner, 2026-09-25: "I wanted openwarrant to be more like beads or jira
than an authority layer … prompting something and having it return a
durable doc and working on that, checking off things so when another agent
came, it only did the items not done." "You just tell an agent to take a
jira ticket and it finishes it. The agent doesn't stop the human every 15
minutes to get it to sign."

Before this Warrant every unit of work in `war` was a Warrant: five atoms,
a compile, an authorization a human signs, evidence, an independent
verification and a resolution a human signs. There was no fast path to
write work down, split it into steps, hand it between agents and finish it
— and `war next` itself takes seconds on this corpus, because every command
reads the whole of it.

## Desired Outcome

- `war create "<sentence>" [--item ...]` makes a ticket that is workable at
  once: two Markdown atoms (an intent; a checklist, one task line per item
  with a short hash id) under `docs/tickets/<t-id>/`, plus a journal.
- `war ready`, `war claim`, `war done`, `war add`, `war note`, `war prime`,
  `war show`, `war tickets`, `war release` work it: no signature, no human
  act, no terminal dialog, each answering in milliseconds, each with `--json`.
- Claims are exclusive (of simultaneous claims exactly one wins; a second
  agent is refused by name), stale ones can be stolen, and all of it is
  journalled.
- The checklist file IS the state: a person's hand edit is honoured, and a
  write moves only the line it touches.
- `war prime` is what an arriving agent or person reads first: remaining
  items, claims, recent notes, done work compacted.
- The authority layer is opt-in: `war check` validates a ticket's structure
  and never reports it for lacking a signature; `war promote` drafts a
  delivery Warrant from a ticket when someone wants sign-off.
- The same loop over `war mcp`; README, QUICKSTART, AGENTS.md and the
  `war init` output start with it.

## Non-goals

- Changing any authority act, signature path, verification or resolution
  rule. A ticket is not a Warrant of the contract corpus and nothing reads
  it as one.
- Sharing claims across machines. Claims are local runtime state; the
  journal carries their history.
- A ticket-level assurance mark. A done ticket is Completion (CONTEXT.md),
  never Verified.

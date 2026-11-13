---
schema: oh.war/atom/v1
adr_uuid: 01a1b7a8-2ed4-7349-a0a7-51f243a0e077
local_alias: OW-ADR-0034
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0948f-2334-7dc2-abe2-0839b7d82e1d"
---

# ADR OW-0034: Who may answer a question follows the ladder

## Status

Proposed, for the owner's signing queue. A performer wrote it from the owner's
decisions of 2026-11-07. It supersedes OW-WAR-0069's rule that only a human answers,
for Tickets and Warrants; for an Order that rule stands.

## Context

OW-WAR-0069 (the hotline) let an agent ask and only a person answer, through
`docs/authority/roles.toml` by actor kind. The ladder came later: an item is a
Ticket, a Warrant or an Order (`CONTEXT.md`), and only an Order needs a person.

In a multi-agent session, an orchestrating agent could not answer its worker's design
question on a Warrant: `war plan answer` refused it as an agent, and the repository had
no roles file at all, so nobody could answer. The question also reached nobody; the
orchestrator had to go looking for it.

The owner decided: "The answer should follow the same level of work, the same ladder
as everyone else", and "Make it so questions are easily picked up and found,
automatically go to orchestrators like humans or other agents, and are done quickly."

## Decision

Who may answer follows the level of the work the question is on:

- **Ticket or Warrant:** any actor except the asker. The answer records who answered and
  their kind, `agent` or `person`. A Warrant's trigger still decides when it is done.
- **Order, or signed text:** a person. A directory Warrant past `draft` counts as signed
  text. An intake question, which asks what a person's input meant, needs a person too.
- **Any level, asked with `--needs-human`:** a person.
- **A level that cannot be read:** a person. The level is never guessed down.
- **The asker answering itself:** refused by name, `question.own`.

Who is a person: with a register (`docs/authority/roles.toml`), the register decides, by
the actor's assignment, and an agent session cannot answer as a person it names. With
none, a person is whoever is at a terminal, with a git identity, in no agent session
(one of the harness session ids `war` already reads). Every other caller is an agent.
An answer's `answered_by` is an attribution, not a proof, as before: a signature binds
an act, and an answer is not one.

Asking names a party. It is the holder of the parent work (`part_of`), else the actor
that dispatched the session (`war start --worktree` records it as
`OPENWARRANT_DISPATCHER`), else the work's creator, else anyone. It is never the asker,
and a question that needs a person is asked of a person. Both halves are W3 notices
(`ticket/live.rs`): the party sees the question in `war status`, `war status --next`,
`war view prime` and Mission Control until it is answered. The asker hears the answer
once. One spelling serves agents and people, `war plan note <id> --ask` and
`--answer`, and MCP `war_note` takes the same arguments.

## Consequences

- An orchestrator answers its workers without a roles file, and without a person in
  the loop for work that never needed one.
- The frontier's `question.no-responder` fires only when a register exists and names no
  person: with none, the person at a terminal can answer.
- A ticket's questions live in `docs/questions/<id>/`, outside the ticket store. An
  answer is written where the question was asked, in another worktree of the same clone
  too. Another clone hears the notice, but answers once the record reaches it.
- A terminal and a git identity are weak evidence of a person. They are as strong as an
  `--as` name in a register, which was the bar before, and an Order still ends in a
  signature that is strong evidence.
- OW-WAR-0069's delivered text says only a human answers. That stays true for Orders.
  For Tickets and Warrants this ADR replaces it.

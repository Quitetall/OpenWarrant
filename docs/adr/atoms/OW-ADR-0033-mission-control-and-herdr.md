---
schema: oh.war/atom/v1
adr_uuid: 01a1b529-0933-7c5b-a8a6-81388980e134
local_alias: OW-ADR-0033
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a1aea7-a3fc-76f2-862d-7f8f8ffed200"
---

# ADR OW-0033: Mission Control, with Herdr as an optional adapter

## Status

Proposed, for the owner's signing queue. A performer wrote it, under t-3c16c W5. It
describes what is on main and decides nothing that is not.

## Context

OW-ADR-0020 chose ratatui for a terminal dashboard. Since then the dashboard has
become where a person watches agents and starts them, which raises two questions that
ADR did not ask: what the app may do by itself, and what it needs from the terminal
multiplexer many of those agents run in.

Herdr runs agents in panes and has a socket API: it can say which agents are alive,
read their terminals, open a tab, start an agent and prompt it. Most of what a person
wants from Mission Control could be built on that, and then Mission Control would not
work without it. It is a separate program with its own release cadence, and a harness
or a person may not be running it.

## Decision

Mission Control is a rendering of the records. It holds no key, keeps no state of its
own beyond the cursor, and every action is a real `war` command that the test suite
parses with the real CLI. A signature stays a child that inherits the terminal.

Herdr is an adapter, and it is optional. Everything in the app works without it, and
what it adds is confined to `herdr.rs` and `tui/herdr_adapter.rs`:

- the Live panel shows an agent's terminal, instead of `git status`, `git diff --stat`
  and the journal;
- Dispatch offers a Herdr tab beside the detached and terminal launchers, for a harness
  Herdr knows;
- a note can be delivered as a prompt now, instead of at the agent's next read.

Herdr is never a source of record. Its calls run off the main thread, a slow socket
never holds a key, and what it cannot answer reads UNKNOWN, not as a shorter fleet. Its
`tokens` are live and die with the pane, so they are not usage (`docs/design/tui-usage.md`).

## Consequences

- Work a person starts from Mission Control is a claim and a worktree whichever launcher
  ran it, so `war` reports it the same way with Herdr or without.
- A harness that needs a terminal gets one from the terminal launcher or from Herdr. A
  detached start of it stops at once, and the log and the standing claim say so.
- The records Mission Control reads are this checkout's. A question asked in a worktree
  shows only once merged, so the app does not yet show everything a running agent has
  written.
- Herdr's API moving can break the adapter and nothing else. Conformance holds this with
  a stub daemon, and a real terminal run of the app needs neither Herdr nor an agent.

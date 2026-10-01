---
schema: oh.war/atom/v1
adr_uuid: 01a0983e-4a12-7b41-8d57-9c1f6e2a4073
local_alias: OW-ADR-0020
role: adr
jurisdiction: bound
order: 30
classification: internal
status: accepted
governs:
  - "war://01a0983e-32db-7240-9a9d-cef5e5b41a38"
  - "war://01a0ca4a-0c02-7cd3-b49d-786377a1aa06"
---

# ADR OW-0020: ratatui and crossterm for the terminal dashboard

## Status

Accepted. The owner authorized OW-WAR-0073 (revision 1) with this ADR in its
basis, and OW-WAR-0112 — which supersedes 0073 and executes the dashboard as
`war` with no arguments — cites it and was authorized on 2026-09-22.

## Context

Two views of the corpus exist and neither is where the work happens: the Pages
projection is rich and lives in a browser that cannot hold a key, and
`war console` acts but shows almost nothing — no corpus state, no obligation
ladders, no receipts, no journal. The owner asked for a full terminal
dashboard on 2026-09-12.

Three ways to build one:

- **Hand-rolled ANSI.** No dependency, and a re-implementation of layout,
  wrapping, scrolling and terminal restoration — the parts that are subtle,
  and the parts a crash exposes by leaving the terminal unusable.
- **crossterm alone.** Input and raw mode without layout; the widgets and the
  diffing renderer would still be ours.
- **ratatui over crossterm.** The layout, widget and frame-diffing layer, with
  crossterm as the backend.

Two constraints bear on the choice. `deny.toml` allows a fixed set of licence
families and a new family is a refusal, not an exception. `OW-ADR-0014` admits
async **only** under `openwarrant-cli/src/mcp/`, so a UI library that needs an
executor would engage that refusal.

## Decision

Take `ratatui` with the `crossterm` backend, confined to
`crates/openwarrant-cli/src/tui/`. A unit test asserts no module outside that
directory names either crate, the same shape as the test that confines `tokio`
and `rmcp` to `mcp/`.

Both crates are MIT, which `deny.toml` already allows, so the dependency adds
no licence family. Both are synchronous: input arrives through a polled
`crossterm::event::read`, so no executor enters the graph and `OW-ADR-0014`'s
refusal stands untouched. `cargo deny check licenses` and a `cargo tree` diff
are the evidence, recorded under OW-WAR-0073 OBL-001 rather than asserted here.

## Consequences

- One new dependency family in the CLI, with a use-boundary test to keep it
  where it was put.
- The dashboard is a **rendering** under SAS 1.1.0 §76.6: it issues commands
  and holds no key. Every act it starts shells out to `war sign --ssh-sign`, so
  the `ssh-add -c` dialog remains the human act. A terminal that signs is a
  defect with a name, which is the point of writing the rule first.
- A panic hook and explicit teardown are part of the deliverable: a library
  that takes over the terminal owes the user their terminal back.
- If a future ratatui release pulls an executor, `OW-ADR-0014` is engaged and
  the version is pinned or the library replaced; the boundary test is what
  makes that visible rather than silent.

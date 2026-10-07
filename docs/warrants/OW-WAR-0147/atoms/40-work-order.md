---
schema: oh.war/atom/v1
warrant_uuid: 01a0da65-9e85-7531-a20e-8e695ecfa9ee
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `profiles/ticket.toml`: the ticket profile — `extends = "delivery"`,
   `form = "working"`, `core_roles = ["intent"]`, requires
   `ticket.checklist` at ordinal 15; `approved = false`.
2. `crates/openwarrant-core/src/role.rs`: `form`/`core_roles` in
   `oh.war/profile/v1`; `ProfileDefinition::working_core_roles`,
   `working_roles`, `is_working_form`; `ProfileError::BadForm` for every
   malformed working form; a core profile has none.
3. `crates/openwarrant-core/src/manifest.rs`: `validate_in` refuses a
   Warrant naming a working-form profile (`WorkingFormProfile`).
4. `crates/openwarrant-core/src/ticket.rs`: the ticket manifest
   (`oh.war/ticket/v1`), hash ids, the checklist parser and writer, faults
   by rule (`ticket.checklist-malformed`, `ticket.item-duplicate`,
   `ticket.blocker-unknown`, `ticket.blocker-cycle`); module in
   `crates/openwarrant-core/src/lib.rs`.
5. `crates/openwarrant-cli/src/ticket/mod.rs`, `claim.rs`, `render.rs`: the
   commands, claims (hard-link lock, steal, release), prime/show/list.
6. `crates/openwarrant-cli/src/lib.rs`: the flattened `TicketCommand`
   subcommands, the plain-`init` start line, `war show`/`war check`
   routing of ticket ids.
7. `crates/openwarrant-cli/src/mcp/tools.rs`, `mcp/mod.rs`: `war_create`,
   `war_ready`, `war_claim`, `war_done`, `war_add`, `war_note`,
   `war_prime`, `war_tickets`; `war_show` renders a ticket; instructions.
8. `crates/openwarrant-cli/tests/tickets_cli.rs`.
9. `conformance/plants.d/45-tickets.sh`.
10. `crates/openwarrant-cli/templates/AGENTS.md.tmpl` and its rendered
    reference `docs/agents/legacy-warrant-workflow.md`; root `AGENTS.md`.
11. `docs/TICKETS.md`, `README.md`, `QUICKSTART.md`, `CONTEXT.md`,
    `docs/PROFILES.md`.

## Frozen Surfaces

The core types OW-WAR-0140 froze (`lifecycle`, `state`, `contract`,
`obligation`, `verification`, `independence`, `resolution`, `authority`,
`deliverable`, `gate`, `gate_run`; compiler `ir`, `canonical`, `digest`);
`show.rs` (pinned by OW-WAR-0033's resolution); every signing, verifying
and resolving path; the `war init --program` output (pinned at three lines
by 99-init and 59-adoption).

## Autonomy and Escalation

Tier T2. Escalate any change that would let a ticket satisfy, skip or stand
in for an authority act, and U-001.

## Rollback

Revert the commits; `docs/tickets/` directories already written stay as
plain Markdown a person can read. A repository with no `profiles/ticket.toml`
keeps working: the tool's built-in copy is the default.

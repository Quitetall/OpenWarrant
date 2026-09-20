---
schema: oh.war/atom/v1
adr_uuid: 01a09762-3fa3-7bf2-bfdc-c86af426c679
local_alias: OW-ADR-0018
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a09762-3fa2-7dd0-b794-0190b44fe879"
---

# ADR OW-0018: Adopt oh.war/inbox/v1 as the machine-readable output of war inbox

## Status

Proposed by a drafting agent through `war plan --apply`; adopted when a human accepts it.

## Status

Proposed

## Context

`war inbox` answers "which Warrants are waiting on a human act". The human-readable table is enough for the operator, but the MCP server, dashboards, and future notification tooling need the same answer in a stable, parseable form. Ad hoc JSON that mirrors internal structs would leak implementation detail and break consumers on every refactor. The project already versions its documents under the `oh.war/<kind>/<version>` scheme and generates schemas from types via `war schemas`.

## Decision

`war inbox --json` emits a document with `api_version: "oh.war/inbox/v1"`, a `namespace` string, a `generated_at` RFC 3339 timestamp, and an `items` list. Each item carries `alias`, `title`, `state`, `awaited_act` (one of the human act names from the classifier), and `waiting_since` (RFC 3339 or null). Items are ordered by wait age descending, then alias ascending. The document is a derived view: it is never written into a record directory. The schema joins the `war schemas` pack and is covered by the transitive digest.

## Consequences

- Consumers get a stable contract; internal struct changes do not leak.
- Adding the schema to the pack changes the transitive digest once; that is expected and recorded.
- Any change to item fields requires `oh.war/inbox/v2` rather than in-place edits.
- The state-to-act vocabulary is now part of a public surface and must be kept in step with the SAS pin (OW-ADR-0016).

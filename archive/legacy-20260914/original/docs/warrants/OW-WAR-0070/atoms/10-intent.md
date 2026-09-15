---
schema: oh.war/atom/v1
warrant_uuid: 01a09762-3fa2-7dd0-b794-0190b44fe879
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The OW namespace now holds more than sixty Warrants. There is no single command that answers the operator's question "which Warrants are waiting on me?". Today the operator must open each record, read its state, and infer by hand whether the next required act is a human one (approve, adopt, answer a blocker, accept evidence, close) or an agent/gate one. This is slow, error-prone, and gets worse with every new Warrant.

## Desired Outcome

A `war inbox` subcommand that reads the record set under `docs/warrants/` and prints every Warrant whose next required act is a human act, one row per Warrant, with: alias, title, current state, the act awaited, and the age of the wait (time since the last transition). Warrants awaiting only an agent or a gate are excluded. The command exits 0 when the inbox is empty and prints a clear "nothing waiting" line. A `--json` flag emits a versioned machine-readable document so other tooling (the MCP server, dashboards) can consume the same answer.

## Scope

- New `war inbox` subcommand with human-readable table output (default) and `--json` output.
- A pure function that classifies a Warrant's next required act as `human`, `agent`, `gate`, or `none` from its current state and pending obligations, reusing the existing state model without changing it.
- Documentation of the command and of the "human act" classification table.
- Unit tests over the classifier and one integration test over a fixture record set.

## Non-goals

- No change to the Warrant state machine, record schemas, or `war check` semantics.
- No notification channel (email, chat, push); this is a pull command only.
- No per-person assignment or multi-user model; "me" means the human operating this checkout.
- No ADR-inbox or roadmap-inbox; Warrants only.
- No sorting/filtering options beyond `--json` in this iteration.

## SAS and Roadmap Traceability

- SAS §74 / §75 (drafting and the human/agent division of acts) motivate the classification of "next required act".
- SAS lifecycle and state sections govern which states imply a pending human act; this Warrant reads them, it does not amend them.
- Roadmap: `roadmap://OW-PHASE-2/plan` (operator ergonomics over the record set).
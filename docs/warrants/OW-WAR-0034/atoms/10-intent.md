---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7188-bfa4-d386a25f4b66
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`openwarrant-agent` contains a `ProposalKind` enum and nothing else. §74.5's rule — an agent never mutates repository files directly — is currently enforced by the crate being empty. That is not enforcement; it is absence.

## Desired Outcome

An agent returns a Draft Proposal of typed atom operations. The proposal is validated against the schema and the semantic rules BEFORE anything is written, and the agent has no path to the filesystem.

## Scope

Agent inputs and outputs (§74.1–§74.2), atom operations (§74.3), validation before application (§74.4), the no-direct-mutation rule (§74.5), and the adapter trait and isolation (§75).

## Non-goals

- No model provider and no agent loop (§79.3). Katana owns the runtime.
- No `war plan` command; that is OW-WAR-0035.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-072` — Complete; governing section in Basis.

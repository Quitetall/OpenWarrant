---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd2-7d30-792c-ba72-6b74daa59f19
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Manifests declare `[[implements]] ref = "sas://WAR-SAS-RQ-014"` and the Warrant
Overview tallies them — but nothing checks the requirement EXISTS. A typo
produces a confident coverage claim for a requirement that was never written.
The Overview already labels its tally "claimed, not verified" for exactly this
reason.

§34.4 also requires architecture-change discovery: work that turns out to change
the architecture must surface a needed SAS revision rather than proceeding.

## Desired Outcome

Every `sas://` reference resolves against the SAS's own §106 index. Unresolvable references fail closed. Requirement status is a separate record from a Warrant's claim (§34.3).

## Scope

Requirement reference resolution, the WAR implementation relation, requirement status as a distinct record, and architecture-change discovery.

## Non-goals

- No verification that a requirement is actually MET; §34.3 makes that a separate
  record, and meeting it is proved by gates (OW-WAR-0020), not by traceability.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-022` — Complete; see Basis for the governing section.

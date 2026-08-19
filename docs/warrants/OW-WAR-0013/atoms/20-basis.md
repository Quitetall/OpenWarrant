---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd2-7d30-792c-ba72-6b74daa59f19
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §34: requirement references, WAR implementation relation, requirement status, architecture-change discovery.

## Prerequisites

OW-WAR-0007 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** §106 is a machine-readable table in the SAS; parsing it
  is mechanical.
- **Blocking unknown.** The SAS is a controlled document that will be revised.
  Whether OpenWarrant parses §106 out of the Markdown, or requires a generated
  index alongside it, is an implementation decision — parsing prose is brittle,
  and a stale extracted copy is worse than parsing.

## Constraints and Invariants

- **An unresolvable `sas://` ref fails closed.** A coverage claim against a
  nonexistent requirement is worse than no claim.
- **A Warrant's claim is not a requirement's status** (§34.3). The two are stored
  separately and the Overview must keep saying so.

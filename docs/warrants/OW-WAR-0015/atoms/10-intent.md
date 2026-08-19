---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf4-72ab-919d-d2b2aeba76b8
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

A Work Order lists deliverables in prose. Nothing types them, so nothing can bind
a produced artifact to the deliverable it satisfies, and nothing computes an
`artifact_digest` — one of the fifteen declared digest domains that has never been
used. §37.2 requires artifact provenance; §37.3 distinguishes a derived report
from the artifact it describes.

## Desired Outcome

Deliverables are typed and content-addressed. An artifact binds to its deliverable by digest, and a derived report can never be mistaken for the artifact it summarises.

## Scope

Deliverable definitions, artifact provenance and digests, derived reports, and performer submission shape (§37).

## Non-goals

- No execution producing artifacts; that is OW-WAR-0024.

## SAS and Roadmap Traceability

No §106 requirement maps here directly; enabling work named in the Production Roadmap.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0ab92-07b6-7223-b0e6-ad5688b18943
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem
The read-only tracker cannot author SDK documents or supply a workflow API.

## Desired Outcome
A separate local reference workflow package serves a browser app and authenticated
HTTP API for versioned Warrant drafts through the existing SDK CLI boundary.
Existing repository Warrant progress remains inspectable without altering history.

## Scope
Create, read, list and revise unverified drafts; preserve exact prior source bytes;
refuse stale revision writes and unauthorized calls; retain state across restart.
Produce a full existing-Warrant reconciliation inventory for release preparation.

## Non-goals
Agent launch, execution claims, human signing, qualification, merge/deploy,
semantic compilation, shared hosting and final Phase 3 qualification are later slices.

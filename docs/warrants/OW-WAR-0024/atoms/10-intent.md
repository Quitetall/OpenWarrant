---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd7-eb6b-7cc7-a7d3-bb8ba8239f96
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

An actor cannot return work. §51 defines the Submission an actor produces and
§51.2 forbids self-completion — the actor says what it did, not that it succeeded.
§52 separates replay, repair, and restart, which today would all look like
'ran it again'.

## Desired Outcome

A stage returns a Submission carrying a performer claim, never a completion. Attempts are typed as initial, replay, repair, or restart, with lineage, and each keeps its own basis.

## Scope

The Submission schema (§51.1), no self-completion (§51.2), performer claim status (§51.3), the four attempt kinds (§52.1–§52.4), and lineage (§52.5).

## Non-goals

- No runtime producing Submissions; that is OW-WAR-0026 and 0027.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-045` — Complete; governing section in Basis.

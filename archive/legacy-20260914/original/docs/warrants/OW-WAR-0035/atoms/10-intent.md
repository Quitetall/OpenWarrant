---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-740b-bb94-32bbdec1cc35
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

A Warrant is authored by hand — every one in this repository was. §99 criterion 1 is that a human can create a draft from one sentence, and criterion 2 that the planner asks only unresolved high-value questions. Neither is possible.

## Desired Outcome

`war plan "<one sentence>"` produces a reviewable, valid draft. The interview asks only what the request left genuinely unresolved.

## Scope

`war plan`, `war interview`, interview generation (§74.6), and the Appendix C interaction shape.

## Non-goals

- No decision-making. The planner drafts; the human authorizes (§27.2 forbids self-authorization).
- No ADR generation; that is OW-WAR-0036.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-071` — Complete; governing section in Basis.

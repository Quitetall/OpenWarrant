---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a1f-771a-a8ed-983d14257cfe
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§98 puts telemetry at Phase 0 — before the compiler — because assurance defaults
and amendment policy are meant to be TUNED from measured distributions (§94), and
without measurement they are guesses. It was skipped.

§95's untracked-work detection is the sharper omission. This repository has
already produced untracked work: the ADR Overview shipped in `3678455` with no
Warrant, and it was found by me reading the log, not by a tool.

## Desired Outcome

Authoring cost, clarification counts, escalations, amendments, gate-failure causes, and untracked work are measured. Commits and artifacts carry WAR identity so untracked work surfaces automatically.

## Scope

The §94 measurement set and derived metrics, and §95 untracked-work detection.

## Non-goals

- No enforcement. §95 is explicit that detection is a diagnostic and governance signal that SHALL NOT fabricate a relationship after the fact.

## SAS and Roadmap Traceability

No §106 requirement maps here directly; enabling work named in the Production Roadmap.

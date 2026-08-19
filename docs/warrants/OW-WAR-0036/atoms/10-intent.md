---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7951-bda1-e484a57f9940
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

RQ-020 says every normative decision is a first-class ADR, and §91.4 test 23 says a normative decision with no ADR FAILS contract validation. Nothing detects one. The two ADRs in this repository exist because I chose to write them; a third decision made silently would leave no trace.

## Desired Outcome

A normative decision without an ADR fails contract validation. The planner proposes an ADR draft when it detects one, and §19.2's execution choices are correctly excluded.

## Scope

Decision detection (§74.7), the normative/execution distinction (§19.2), ADR creation during execution (§19.5), and proposed-ADR generation (RQ-073).

## Non-goals

- No judgement about whether a decision is GOOD. Detection is about whether a choice is normative, not whether it is right.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-020` — Complete; governing section in Basis.
- `WAR-SAS-RQ-073` — Complete; governing section in Basis.

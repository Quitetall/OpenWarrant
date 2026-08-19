---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd4-ab16-7823-bf23-99904a35aea0
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`war check` currently satisfies RQ-055 with `text.contains("adequacy")`. Any
assurance atom containing that word passes. It is a check-shaped non-check, and
it is in the shipped binary.

§39 requires an adversarial question with recorded OUTCOMES and EXECUTED ATTACKS
(§39.3) — not a section heading.

## Desired Outcome

Adequacy review is a structured record: the adversarial question, its outcomes, the attacks actually executed, and an honest statement of limitation. The substring search is deleted.

## Scope

The adequacy review record (§39.1–§39.5) and its enforcement at controlled and high assurance.

## Non-goals

- No automated adversarial testing. §39 asks a human to attack the gate design; this records the result.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-055` — Complete; governing section in Basis.

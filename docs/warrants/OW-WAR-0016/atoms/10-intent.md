---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf5-7161-9490-5d957bdf0d7d
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

This repository has 21 acceptance obligations. Every one is prose in an assurance
atom, and exactly one is checked by anything — `text.contains("adequacy")`, a
substring search.

§38.1 is explicit that a completion summary decomposes into obligations rather
than being one claim, and §38.4 requires a universal claim to declare its scope.
Our obligations already do both, carefully, by hand. Nothing enforces it and
nothing aggregates them into a resolution.

## Desired Outcome

Obligations are typed records with bounded scope and dispositions. A universal claim that does not declare its scope is refused. Resolution aggregates dispositions rather than asserting a verdict.

## Scope

The obligation schema (§38.2), scope kinds (§38.3), universal claims (§38.4), dispositions (§38.5), and resolution aggregation (§38.6).

## Non-goals

- No gate binding; obligations reference gates, and Gate Definitions are
  OW-WAR-0019.
- No evidence; that is OW-WAR-0017.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-050` — Complete; governing section in Basis.
- `WAR-SAS-RQ-051` — Complete; governing section in Basis.

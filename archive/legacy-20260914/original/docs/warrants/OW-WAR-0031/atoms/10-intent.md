---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b8-7209-89bb-f36dde06b52c
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

State has nowhere to live. OW-WAR-0008 must DERIVE state because nothing records transitions, so no transition can be audited. §66 gives an append-only local journal for exactly this: drafting and milestone provenance before federation exists.

## Desired Outcome

Material events are journalled append-only. State becomes recorded rather than derived, and a transition has a history.

## Scope

The journal's purpose and authority (§66.1–§66.2), event envelope (§66.3), and material events (§66.4).

## Non-goals

- The journal is NOT authoritative (§66.2). KF is, once registered. This is local provenance.

## SAS and Roadmap Traceability

No §106 requirement maps here directly; enabling work named in the Production Roadmap.

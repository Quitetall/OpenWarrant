---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf4-7b46-8ed4-7e9634593bec
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Rationale lives in prose. §35 models it as a graph — facts, priorities,
forecasts, alternatives, decisions, with typed edges — precisely so a reader can
ask which fact supports which decision. §36 separates an evidenced premise from
an accepted residual risk from a blocking unknown, and §36.4 forbids circular
validation: a premise validated by the thing it justifies.

Today all three classes are written as bullet points I chose by hand.

## Desired Outcome

Rationale is a typed graph. Assumptions carry their class, blocking unknowns block, and circular validation is detected rather than argued about.

## Scope

Rationale node classes and edges (§35), the three assumption classes (§36.1–§36.3), circular-validation detection (§36.4), and claim narrowing (§36.5).

## Non-goals

- No inference. The graph records reasoning; it does not evaluate whether the reasoning is good.

## SAS and Roadmap Traceability

No §106 requirement maps here directly; enabling work named in the Production Roadmap.

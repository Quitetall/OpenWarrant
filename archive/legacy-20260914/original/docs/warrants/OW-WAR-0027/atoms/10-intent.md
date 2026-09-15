---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-714f-973c-6decbf23e65d
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

BLUT owns typed computational execution and lineage (RQ-063). A computational stage cannot reach it: there is no lowering from a WAR stage graph to a BLUT PlanSpec, and no consumption of BLUT's lineage receipt. Unlike Katana, BLUT IS available on this host at `training/engine`, so this seam can be exercised.

## Desired Outcome

A computational stage lowers to a BLUT PlanSpec and its lineage receipt is consumed, without OpenWarrant duplicating BLUT's DAG, scheduler, or lineage model.

## Scope

Lowering (§49.1), adapter duties (§49.2), and the authority boundary (§49.3).

## Non-goals

- No DAG execution, no scheduling, no lineage computation. All three are BLUT's (RQ-064).

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-063` — Complete; governing section in Basis.

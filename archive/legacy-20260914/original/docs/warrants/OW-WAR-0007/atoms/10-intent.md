---
schema: oh.war/atom/v1
warrant_uuid: 01a01bcf-b0e1-71e3-a237-3554610254d3
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Every Warrant declares a `45-milestones.yaml`. Its bytes are hashed and it renders
into the parent. Nothing reads it. `grep` for `stage_refs`, `executor_kind`, or
`responsibility_tier` across the crates finds them only in the TEMPLATE WRITER.

So RQ-040 — "milestones and stages are distinct" — is true of the documents and
unenforced by the tool. A Warrant can reference a stage that does not exist, or
declare a milestone graph with a cycle, and `war check` reports well-formed.

## Desired Outcome

Milestones and stages are typed values. Dangling `stage_refs` and
`obligation_refs` fail closed, the milestone dependency graph is acyclic, and
stage ports are named and typed per §23.5.

## Scope

The `oh.war/milestones/v1` schema: milestones, dependencies, stages, executor kinds, responsibility tiers, and named typed ports.

## Non-goals

- No execution. A stage being well-formed says nothing about running it.
- No Dispatch compilation; that is OW-WAR-0023.
- No milestone STATE — whether a milestone is met needs the state model
  (OW-WAR-0008) and gate runs (OW-WAR-0020).

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-040` — milestones and stages are distinct. Complete.
- `WAR-SAS-RQ-041` — stages use named typed ports. Complete.

---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd6-ebf5-77d8-b498-a397823907f3
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §47: Dispatch schema, Dispatch compilation, actor-specific projection.

## Prerequisites

OW-WAR-0007, OW-WAR-0011, OW-WAR-0012 resolved — a Dispatch carries the stage, the readiness, and the context.

## Assumptions and Unknowns

- **Evidenced premise.** §47.1 gives the schema in full.
- **Accepted residual risk.** 'Everything one actor needs' is judged by the
  compiler; an under-projected Dispatch fails at execution, not at compilation.

## Constraints and Invariants

- **A Dispatch is self-contained** (RQ-042). A stateless actor gets ONE Dispatch,
  not a Dispatch plus a repository.
- **Authority is bounded and explicit** (RQ-044, §30). A Dispatch says what the
  actor may do, and omission is denial.
- **`dispatch_digest` is a declared domain.**

---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-7118-8a05-0e8bf46ef650
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Nothing can talk to Knowledge Fabric. §67 requires state change through TYPED ACTIONS rather than direct status edits (RQ-076) — the rule that stops a client from writing a lifecycle field directly. KF is checked out locally at `/mnt/4tb/openhuman-knowledge-fabric`, so this is buildable and partly testable.

## Desired Outcome

OpenWarrant changes federated state only through typed KF actions, with an action envelope carrying optimistic concurrency and idempotency.

## Scope

The controlled-action vocabulary (§67), the envelope (§67.1), server time (§67.2), optimistic concurrency (§67.3), and idempotency (§67.4).

## Non-goals

- No registration or identity allocation; that is OW-WAR-0029.
- No reimplementation of KF's action kernel — it is TypeScript and stays there (§77.3).

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-076` — Complete; governing section in Basis.

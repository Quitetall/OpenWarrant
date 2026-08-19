---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-7118-8a05-0e8bf46ef650
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §67 (controlled actions, action envelope, server time, optimistic concurrency, idempotency), §83 (KF integration).

## Prerequisites

OW-WAR-0008 and OW-WAR-0022 resolved — actions transition state and record resolutions.

## Assumptions and Unknowns

- **Evidenced premise.** KF is local and its action kernel exists at `packages/actions`.
- **Blocking unknown.** §83.4 says TypeScript consumes GENERATED schemas rather than reimplementing WAR semantics. Schema generation is OW-WAR-0032, so the direction of the contract must be settled first: OpenWarrant generates, KF consumes.

## Constraints and Invariants

- **Typed actions only** (RQ-076). No direct status edits, ever.
- **Server time is authoritative** (§67.2). A client clock never stamps a controlled action.
- **Optimistic concurrency** (§67.3): a stale write is refused, not merged.
- **Idempotency** (§67.4): a retried action does not double-apply.

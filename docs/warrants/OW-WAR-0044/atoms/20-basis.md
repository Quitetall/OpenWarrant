---
schema: oh.war/atom/v1
warrant_uuid: 01a021a4-be73-76f7-9aa7-d883cc39d51e
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

§67, §83, §12, §68, §91.3 tests 19 and 22, §91.13 tests 91–95.

## Prerequisites

OW-WAR-0028, OW-WAR-0029 and OW-WAR-0030 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** Knowledge Fabric is checked out on the
  development host and is buildable, so this is the most actionable of the live
  integrations.
- **Blocking unknown.** KF has no running instance with an allocator, and
  OW-ADR-0004 recorded that the OpenHuman Identifier Registry does not exist yet.
  *Resolution requirement:* a reachable KF instance that will allocate an
  identifier under the registry and return it in a receipt.
- **Accepted residual risk.** Registration is tested against one instance. A
  second instance could disagree about identity semantics.
  *Consequence if false:* two instances allocate colliding identifiers and the
  word "global" stops meaning anything.

## Constraints and Invariants

- **KF allocates; nothing else may** (§12.4). The manifest validator
  already refuses a locally-derived identifier, and the check is on PROVENANCE,
  not shape — a fabricated identifier always looks right.
- **Git remains Source Holder** (§91.3 test 21). Registration adds authority; it
  does not transfer it.
- **The server assigns `recorded_at`** (§67.2), version drift FAILS rather than
  overwrites (§67.3), and a reused idempotency key with a different payload is
  rejected (§67.4).

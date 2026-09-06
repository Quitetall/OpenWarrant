---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3eb6-7ac2-a2d2-1fac35901cdd
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/wmd-sim/src/effects.rs` typed effect records, bounded buffer,
   canonical ordering, atomic validation, property world, and domain events.
2. `docs/work-packages/WP-008-effects-events.md` package contract.
3. WMD commit `67b98c4`; local `./scripts/ci.sh` receipt.

## Frozen surfaces

ADR-0013 sort key and commit phase ordinal 6; typed `PropertyValue`; no partial
atomic-group publication; 1,024-record, 256-group-member, and 4 KiB payload
bounds.

## Execution and rollback

Run formatter, full local CI, duplicate-key, wrong-phase, ordering, and atomic
refusal tests. Roll back by reverting subject commit; hosted CI is optional.

## Autonomy and escalation

T1 implementation inside accepted ADRs. New effect operators, payload types, or
authority policy require owner ADR review. Performer may not authorize, verify,
or resolve this Warrant.

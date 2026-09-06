---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3eac-7bf2-92ec-9fdddfa15045
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/wmd-sim/src/actions.rs` action definitions, bounded timeline,
   transitions, interruption, and refusal faults.
2. `crates/wmd-sim/src/scenario.rs` H2 lifecycle exercise.
3. `docs/work-packages/WP-007-action-timelines.md` package contract.
4. WMD commit `67b98c4`; local `./scripts/ci.sh` receipt.

## Frozen surfaces

ADR-0002 phase order, ADR-0011 capability legality, stable ID semantics, and
the 1,024 action bound. No HPR or renderer dependency may enter these files.

## Execution and rollback

Run `cargo fmt --all`, `./scripts/ci.sh`, and the H2 CLI proof. Roll back by
reverting the subject commit; no hosted CI or cloud service is required.

## Autonomy and escalation

T1 implementation inside accepted ADRs. Any new action authority, cost policy,
or schema revision escalates to owner ADR review. Performer may not authorize,
verify, or resolve this Warrant.

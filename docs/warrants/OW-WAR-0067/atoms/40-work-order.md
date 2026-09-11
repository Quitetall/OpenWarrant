---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3ebf-7ae3-982e-828776450229
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/wmd-sim/src/scenario.rs` bounded command validation, H2 runner,
   replay comparison, divergence report, v2 checksum, and smoke test.
2. `apps/wmd-sim-cli/src/main.rs` `--h2` headless proof output.
3. `docs/work-packages/WP-009-scenario-replay.md` package contract.
4. WMD H2 implementation/evidence commits through `bc38378`; local gate and
   lamu review receipts.

## Frozen surfaces

H2 exit contract; monotonic commands; 4,096 command bound; v2 checksum domain;
no renderer, network, or HPR input.

## Execution and rollback

Run `./scripts/ci.sh`, `wmd-sim-cli --h2 --seed 7`, and 2,048-seed smoke.
Rollback by reverting subject commits. No hosted CI or cloud service required.

## Autonomy and escalation

T1 implementation inside accepted ADRs. Full envelope persistence, migrations,
or release identity changes escalate to owner ADR review. Performer may not
authorize, verify, or resolve this Warrant.

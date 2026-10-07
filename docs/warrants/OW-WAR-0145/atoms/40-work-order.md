---
schema: oh.war/atom/v1
warrant_uuid: 01a0d31e-63a1-7092-a4d2-0937d817adcb
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `conformance/plant-isolated.sh`: clone `HEAD` into `mktemp -d`, link
   the clone's `./target` to the repository's resolved target directory,
   export `CARGO_TARGET_DIR` to it, run `bash conformance/plant.sh` there,
   print its output and a final `tested commit <sha>` line, remove the
   clone on every exit path, and exit with the battery's status. Refuse
   (`exit 2`, a named line) when `OPENWARRANT_IN_BATTERY` is set, and set
   it for the battery it runs. `OPENWARRANT_ISOLATED_PLANT_SH` may name a
   different battery script inside the clone, for the plants below only.
2. `docs/gates/ops.conformance.plants@1.1.0.yaml`: the same gate id at
   version 1.1.0, argv `["bash", "conformance/plant-isolated.sh"]`,
   `mutating: "false"`, `timeout_secs: "3600"`, the fault model and blind
   spots of `@1.0.0` plus "tests the committed tree, not uncommitted
   work", and qualification fields naming this Warrant's plants.
3. `conformance/plants.d/48-isolated-battery.sh`, on a scratch corpus that
   commits its own copy of `plant-isolated.sh`, a copy of the `@1.1.0`
   definition, and a fake battery script (`OPENWARRANT_ISOLATED_PLANT_SH`)
   — never the real battery, so nothing recurses.

## Frozen Surfaces

`@1.0.0`'s definition; `conformance/plant.sh` and `lib.sh`; every plant;
`gate_cmd.rs`'s askability rule.

## Autonomy and Escalation

Tier T2. Escalate rather than decide any change to `askability_of` or to
`@1.0.0`.

## Rollback

Delete `@1.1.0` and the script. Receipts minted under it stay as history.

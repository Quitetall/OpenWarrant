---
schema: oh.war/atom/v1
warrant_uuid: 01a0d31e-63a1-7092-a4d2-0937d817adcb
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`gate://ops.conformance.plants@1.0.0` is the gate most obligations in this
corpus cite — 28 unresolved Warrants, and OW-WAR-0066 before them. It can
never produce a receipt. It declares `mutating: "true"` (the battery rewrites
the working tree while it runs), and `gate_cmd.rs`'s `askability_of` refuses
every mutating gate "regardless of how well it is declared" (§44.8). Under
`war evidence record` and under `war run` of a service stage alike, the run
is recorded `not_askable (mutating)`, and §56.1 requirement 5 cannot be met
for any obligation that cites it.

The gate's own definition says "A stage bound to this gate runs
`conformance/plant.sh` under `war run`"; that was never true. OW-WAR-0066
recorded the binding for an example only, and the 2026-09-23 amendments
(AM-001 on 27 Warrants) added service stages for a gate the tool refuses.
Found on 2026-09-24, when OW-WAR-0115's stage was run.

## Desired Outcome

`gate://ops.conformance.plants@1.1.0`: the same battery, run in a
disposable clone of the committed tree, so the repository's working tree is
never touched and the gate is truthfully `mutating: "false"`, askable, and
able to mint a §44.6 receipt.

- The run tests `HEAD` — the committed tree — and the receipt says which
  commit. An uncommitted change is not tested, and the gate says so rather
  than implying otherwise.
- The clone is created under the system temp directory and removed after
  the run, pass or fail. The working tree, its index and its `HEAD` are
  byte-identical before and after.
- A battery nested inside the battery refuses by name instead of
  recursing.

`@1.0.0` stays as it is: a gate version is governed and never rewritten
(RQ-056). The Warrants that cite it move to `@1.1.0` by amendment, a
separate act for the owner.

## Non-goals

- Making any mutating gate askable, or relaxing §44.8.
- Changing the battery's plants or what they test.
- The amendments that move citations to `@1.1.0`.

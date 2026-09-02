---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1f2d-75b3-8b0e-2a62c2acab59
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work order

Ordered. Each step states how it is verified; a step without a check is not done.

## 1. Re-anchor `ci_local.GATE_GLOBS` BEFORE any wave

The globs are root-anchored, so merged submodules' gate tools stop being seen by
`check_coverage()` the moment a wave lands, and the wiring ratchet stays green
while covering less. Fix first, then prove it can fail: plant a gate-shaped tool
under a merged path and require `--check-coverage` to report it.

Verify: `python3 tools/ci_local.py --check-coverage` refuses the planted tool,
then passes once declared.

## 2. Repair the KF `lamquant-compat` oracle for the post-collapse layout

`CODE_PREFIXES`, `REQUIRED_TOOL_PATHS`, and the gitlink reading in
`git-materialization.ts`. This must land WITH wave 1, not after: a missing
gitlink makes the materialisation loop `continue` **silently**, so the oracle
verifies less than it claims and reports success for it.

Verify: `pnpm --filter @kf/documents test lamquant-compat` green against the
13-repo layout first (that is the §97.5 parity baseline — without it there is
nothing to compare the collapsed tree against), then green after each wave.

## 3. Waves, cheapest-first, `josh-filter ':prefix=<path>'` only

One wave per commit. Never `git-filter-repo` — see intent.

Verify per wave: `python3 tools/scripts/verify_projection.py --component <name>
--strict --scratch-dir /mnt/4tb/tmp` AND a green oracle run. Capture exit codes
without a pipe; `$?` after `| tail` reports `tail`.

## 4. Retire the pins the collapse makes dead

88 `rev =` pins over 9 URLs at authoring (ABIR 47 · blut 13 · Lossless 12 ·
LamQuant 7 · liblsl-rust 3 · Firmware 2 · blut-backends 2 · OpenECS 1 · Vision
1). The collapse deletes roughly 69. blut, openecs and liblsl pins SURVIVE —
this deletes pins to merged components, not the pin system.

Verify: `python3 tools/ci_local.py --job pins --job locks` exit 0, and
`tools/scripts/integrate.py --apply` converges in one round.

## 5. Close the three exposure edges

- `lossless -> meta` closes with the wave that merges them.
- `eagle-lqs -> blut-engine` and `legacy -> vision` close by consuming
  PUBLISHED versions instead of private rev-pins. blut is already on crates.io
  as `blut-graph-core 0.1.0-alpha.1`, which is the precedent that closed
  `lossless -> blut-engine` on 2026-08-20.

## 6. Lower the ceilings to the new measured values, never to a round number

`tools/module-ceilings.json` may only decrease, and only to what was observed.
Add the dated note the file's convention requires, naming what closed each.

Verify: `python3 tools/check_module_direction.py --strict` and
`python3 tools/check_module_exposure.py --strict` both exit 0 reporting 0/0/0.

## 7. Only then, append 0185's completion

0185 is append-only and living. Do not rewrite earlier text.

## Rename, last

`Quitetall/LamQuant` -> `Quitetall/OpenHuman-Technologies-LamQuant`. Repository
rename only, NOT an org move — "OpenHuman Technologies LamQuant" contains spaces
and GitHub names cannot, and an org move is a 10x blast radius. Do it after step
4 has already deleted the 7 `LamQuant.git` pins. Also update `modules.toml`
`repo` fields, `contract.toml` owners, the SAS's cited basis row, and KF's
oracle config.

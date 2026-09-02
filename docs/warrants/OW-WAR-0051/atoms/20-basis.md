---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1f2d-75b3-8b0e-2a62c2acab59
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

Measured in `/mnt/4tb/LamQuant` on 2026-08-26. Re-measure before acting; these
are the state at authoring, not a standing guarantee.

## The ratchets

`tools/module-ceilings.json` — ceilings may only DECREASE:

    max_layer_inversions   1
    max_exposure_edges     3
    max_exposure_pin_sites 3

Measured, at ceiling on all three:

    [layer] lossless(L1) -> meta(L3)
        codec-lossless/lamquant-lossless/Cargo.toml:324  lamquant-tui [dependencies]
    [exposure] lossless (public)   -> meta (private)          same line
    [exposure] eagle-lqs (public)  -> blut-engine (private)   evaluation/eagle-lqs/cookbook/Cargo.toml:18
    [exposure] legacy (public)     -> vision (private)        legacy/crates/lamquant-runtime-legacy/Cargo.toml:63

55 cross-module production edges total, 0 modules in a cycle. A cycle is never
ratcheted.

## What the lossless edge actually costs

From 0185's Progress Log, measured 2026-08-26:

- ONE direct dependency: `lamquant-tui`, **optional**, behind codec-lossless's
  non-default `tui` feature. `default = ["host"]` does not pull it, so only
  `--features tui` and `--all-features` are broken for outside users.
- Closure is seven crates, ~39,300 LoC: lamquant-tui 15,121 · lamquant-runtime
  13,362 · lamquant-ops 3,649 · lamquant-config 1,908 · lamquant-config-schema
  1,629 · lamquant-history 935 · `tui/` (the launcher) 2,701.
- `tui/` is in the closure through a test-only edge invisible to Cargo:
  `lamquant-ops`'s `operation_contract.rs` does
  `include_str!("../../../tui/src/op_control.rs")`.

## What changed on 2026-08-26, and what did not

The codec's `src/tui/mod.rs` no longer carries `pub use lamquant_tui::*`
(codec-lossless d1ffd85). The glob was holding up two of the codec's own test
files — `reducer_unit.rs` (10 tests) and `config_save.rs` (3) — neither with a
single codec reference across 240 lines; both relocated to
`crates/lamquant-tui/tests/` with counts preserved.

The edge is now one function wide: `bin/lml.rs` calls `tui::run_interactive()`,
and `src/tui/manifest.rs` names `ShellManifest`, `PanelRegistration`, `TileSpec`
and four `router::SCREEN_*` constants. `cargo check -p lamquant-lml
--all-targets --locked` exits 0.

**The ratchet did not move and was not expected to.** Width changed; existence
did not. Do not read the narrowing as progress toward the count.

This also expired the premise of 0185's own "NOT ASSESSED" note, which doubted
the `lamquant-plan` carve-out precedent would transfer *because* the codec
consumed the whole shell. It no longer does. Whether a carve-out is viable is
still open; only the stated reason for doubting it is gone.

## Remaining reacher

`codec-lossless/lamquant-lossless/tests/cli_contract.rs` still names
`lamquant_tui::state::AppState` and `lamquant_tui::operations::PlanProjection`.
The projection half can likely retarget the in-repo `lamquant-plan`, where that
vocabulary now lives. `AppState` is genuinely framework state with no public
equivalent.

## Consolidation constraints

- Waves cheapest-first, each gated by `tools/scripts/verify_projection.py
  --component <name> --strict --scratch-dir /mnt/4tb/tmp`.
- **blut stays a separate repository**, consumed by crates.io version — highest
  fan-in in the fleet (7 modules), the only real release cadence, ADR 0034's
  domain-neutral charter.
- `modules.toml` has `[module.meta] path = "." merge = "root"`, so
  `docs/decisions/` never relocates. There is no relocation step for the corpus.
- `ci_local.GATE_GLOBS` is root-anchored. After the collapse, twelve submodules'
  gate-shaped tools become invisible to `check_coverage()`, and
  `wiring-ceilings.json {orphan_tools 0, unexecuted_tools 0}` stays green **while
  lying**. Re-anchoring is a required step, not a follow-up.

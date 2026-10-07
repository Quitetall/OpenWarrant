# Full repository gate result

Command: `CARGO_TARGET_DIR=/mnt/2tb/ow-closure-audit-target cargo +1.97.1 xtask gate`
Source commit: `91f912aa`. Terminal process exit: 1.
Raw output: `/mnt/2tb/ow-telemetry-json-gate-91f912aa.log`
Raw output SHA-256: `edad0b9bf66f9843a4d2f2f05353893715e0bbbd95565965fe2e43e1093b4bd1`

Five of fourteen steps failed:

- SPDX headers: five inherited missing license comments; fixed in integration `093900a5`.
- Workspace tests: corpus JSON payload differed from the stale committed projection.
- Licenses: `cargo deny` was unavailable, so license compliance was not established.
- Corpus: five stale generated projections; regenerated in integration.
- Plants: corpus freshness failed; battery later aborted with exit 9 at the contractor plant's frozen-module copy setup. This is not a completed battery.

Formatting, Clippy, schemas and attestations passed on this source. This does
not qualify the integrated branch. Its generated check passes after compilation,
and the isolated complete 69-current plant passes 31/31. The whole integrated
repository gate must run after fixing the unavailable license checker and
contractor plant setup. No independent verification, signing or resolution.

# OW118 native-package friction observations

Performer observations, not qualification or a new baseline.

The Linux package from [release rehearsal 38149192266](https://github.com/Quitetall/OpenWarrant/actions/runs/38149192266) was downloaded, its archive checksum verified, and its actual binary executed: `war 1.0.0-alpha.3 (unreleased, commit 47cc456348ae)`. The measurement used `tools/friction/measure.sh --runs 3 --corpus <this checkout>` at procedure/corpus source `e7b853275580f3696f830d75685826efaa9ad73c`. All tool steps returned their declared accepted exits. A refused resolution remains `not_ready`, rather than success.

Tool-only medians: setup 314 ms; scratch routine sequence 336 ms; fixture-backed intake sequence 121 ms. The record counts three setup, one routine and two intake human steps as `not_measured`. The intake drafter is a fixture, so no model latency, cost or real drafting performance is established. Neither the ten-minute setup target nor the sixty-second administration target is proved.

The script reports build profile UNKNOWN: the packaged ELF lacks debug information, but the path is not a build provenance receipt. That honest output is preserved; it is not changed to satisfy the release-baseline obligation. The hosted release workflow separately records a release build. Its package manifest labels source linkage caller-supplied/unestablished and qualification false.

Hosted CI source `47cc456348ae0ebe4291177647b6f563fb191c21`, run 38148947355, passed eleven friction controls: timing/counting, refusal of timed human steps, cleanup and socket restoration, failed-step UNKNOWN propagation, baseline structure/order, document-row agreement and rejection of an altered median. Existing historical baselines remain unchanged. A rerun of the exact historical alpha.2 baseline binary, independent review and formal resolution are not established here.

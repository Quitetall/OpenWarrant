# Explicit local verifier route for preparation

`war prepare` can select experimental compact v3 inputs and a verifier command for one run with repeated `--verifier` arguments. The child verification uses the same deadline, exact packet/view metadata and ingestion checks as configured execution. Repository configuration remains unchanged. The default command/transport stay the configured argv and v2. A dry run describes the explicit selection and does not start an adapter.

The public routing test first failed because `prepare --compact` was unsupported. It now passes with a synthetic standalone service and the configured default deliberately set to `false`. The control checks v3 records, unchanged config and absence of human resolution. No live provider was called by these controls.

A second public control reproduced a real false-completion bug: after existing current v3 verdicts, a forced v2 re-review returned an unsupported future protocol (UNKNOWN), but prepare reported verify done, 2/2 established and exit 0. Preparation now preserves the child UNKNOWN, exits 2 and stops that Warrant without rewriting earlier records or journal. The initial malformed test envelope was corrected before this behavioral red was obtained; both logs are retained.

Rust 1.97.1: all 61 verification CLI regressions and all-target CLI Clippy pass. Full preparation controls, aggregate CLI tests and full CI remain to run at this checkpoint. Actual corpus preparation has not run. The 116-Warrant dry run and its 54 missing declarations, 11 planned refreshes and 21 human authorization acts are structural planning evidence only. No production verdict, human act, signature, Warrant closure, or full-wave completion is claimed.

Raw controls: `/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/prepare-local-verifier-20261016/`.

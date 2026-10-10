# Current Phase 1 native check — 2026-10-10

Native run [38070467635](https://github.com/Quitetall/OpenWarrant/actions/runs/38070467635)
measured main `a9488ab0555ebc9eec04bde37d53ce16d985ad22` on Linux and macOS.
Both receipts bind the same complete source inventory. Both failed only the CLI
check: Cargo refused `--test sdk_cli` because integration tests were consolidated
into the `all` target. All other checks, including source and binary stability,
passed on both hosts. Historical passing receipts are preserved, not relabeled
as observations of current main.

The runner now invokes `--test all sdk_cli::`, the command documented by the
registered integration module. It runs the same SDK suite rather than deleting
or changing an acceptance expectation. Code fix: `f5eba6f8690b4cb6b41161fb1b0616d7294a594e`.
The actual selected CLI suite passed all 11 tests on Rust 1.97.1, and all four
inventory/stability tests passed. A fresh native run of the corrected candidate
is still required; no Phase 1 exit or independent qualification is claimed.
Signed atoms, authorization and scope records remain unchanged.

`comparison.json` records bounded host results and receipt hashes. Full native
receipts remain in the Actions artifacts and at
`/home/brianklam/Projects/OpenWarrant/docs/runtime-evidence/native-kf-recovery-20261010/phase1-current-linux/`
and the adjacent `phase1-current-macos/` directory. These are failed observations,
not verifier dispositions or human acceptance.

The corrected native run [38071530139](https://github.com/Quitetall/OpenWarrant/actions/runs/38071530139) passed on Linux. macOS reached the same selected target and failed to compile a separate collector test: `rustix::fs::mkfifoat` is unavailable on that target. The FIFO fixture now uses the Unix `mkfifo -m 600` command, as the existing progress-viewer fixture does. The assertion still requires refusal of non-regular enrollment files. All three collector-loading tests passed locally on Rust 1.97.1; the next macOS run must establish portability. No hosted pass or phase qualification is claimed.

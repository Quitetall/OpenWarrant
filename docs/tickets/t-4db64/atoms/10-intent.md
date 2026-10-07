# OW-WAR-0148 M1: one compiled corpus model

## Notes

- **2026-10-03 01:56 UTC, claude:** Differential (baseline d2171045 vs this branch), 8 commands: byte-identical stdout+exit on a fresh clone of the branch (debug) and on the 1000-Warrant synth corpus (release, 500 resolved). The state fix shows on neither (every resolved Warrant has a journal); demonstrated on a copy with SC-WAR-0002's journal removed: baseline overview draft, status resolved; new overview resolved. Budget (release, 1000): check 3.2s, check --generated 4.5s (was 8.4), next 2.3s (3.2), status 0.4s, pins 0.8s, sign --list 4.0s (4.3), compile 3.2s (7.1), console --json 3.6s (5.1), console 5.1s (6.6); all within. war model 0.55s, 2.0 MB, deterministic. cargo test --workspace, clippy (+schema), war schemas --check pass. Deliverable 5 (prepare shares one run) NOT built: receipts are per Warrant through evidence.rs/gate_cmd.rs, outside this Warrant's declared set.

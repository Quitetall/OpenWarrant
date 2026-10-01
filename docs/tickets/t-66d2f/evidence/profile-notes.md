# Scoped profile observations

Performer observations by codex, not independent assurance.

- Earlier exact candidate `da924b07`: full local gate passed all 14 steps and 1,275 battery checks. External log `/mnt/2tb/ow-pr136-da924b07-full-gate.log` was created at 2026-09-30 19:28:26 EDT and last written at 20:03:43 EDT. This is whole gate elapsed time, not a measured per-plant duration.
- Actual `check --generated --json` trace: 185 Git processes, summed Git elapsed time 2.895 seconds, whole command 23.318 seconds on a cold/busy checkout. Raw trace `/mnt/2tb/ow-gate-source-trace.jsonl` and response `/mnt/2tb/ow-gate-source-check.json` are retained outside the repository.
- Two subsequent `check --json` observations: 5.422 seconds (4.651 user CPU, 0.734 system CPU), then 5.341 seconds (4.552 user CPU, 0.742 system CPU). Both response digests were `b9d6abf731ff8c85331e30355ade5cfc6b76eaf88f35cc1aae90136a5f53c88d`. Responses remain at `/mnt/2tb/ow-gate-warm-check-0.json` and `-1.json`.
- Actual failing overview control: 1,138 tree scans, six distinct queries. Its full assertion is retained in `overview-tree-red.log`.
- One-shot memo alone passed the scan control but still failed the actual HTTP deadline, retained in `project-http-scan-only-failure.log`.
- Reusing the already observed corpus for the roadmap then passed the actual HTTP test in 4.099 seconds. The deadline remains ten seconds; this is a local observation, not a universal latency promise.
- Final trace: six tree scans once each; index bytes unchanged. Eight viewer tests pass, including live refresh after edits, stale last-good state on invalid source, unknown reports and access refusals.

The full CI budget remains unestablished. No cache crosses commands or refreshes. No mandatory expectations, required checks, signatures or provider receipts were substituted.

## SHA-256 dependency profile experiment

The observed `check --json` stack at 2.5 seconds was inside `sha2::sha256::x86_sha::compress`, reached through `check_deliverable_digests` and `sha256_hex` (external diagnostic `/mnt/2tb/ow-cpu-sample-2.5.log`). The workspace dev profile now optimizes only the existing sha2 dependency at level 3. This changes neither the hash algorithm, canonicalization, dependency version, nor workspace debug assertions or overflow checks. Cargo test inherits dev's dependency override ([Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)).

Six balanced pairs compared standalone baseline `/mnt/2tb/war-overview-shared-status` and optimized `/mnt/2tb/war-sha2-optimized` binaries against the same working tree. All twelve calls exited zero with byte-identical reports. Median elapsed time was 2.7195 seconds baseline versus 2.4490 optimized; median user CPU was 2.2600 versus 2.0195 seconds. These are scoped warm local observations. Compiler tests ran concurrently during part of this comparison; this is not an isolated machine benchmark or universal bound. Raw observations are in `sha2-balanced-comparison.json`.

All 76 compiler tests passed, including SHA-256 published vectors. A disposable clone of f6c5e398 exercised the actual CLI with optimized sha2: valid corpus passes; altered OW-WAR-0061 digest is refused by `deliverable.digest-drift`; missing target is refused by `deliverable.target-unreadable`; restoring the original record returns the byte-identical passing report. See `sha2-refusal-results.json`. The live signed record was never changed.

Exact 8813f541 local gate completed all 14 steps with 1,276 battery checks passing (`/mnt/2tb/ow-pr136-8813f541-full-gate-layout-fixed.log`). Hosted web run 36797364835 passed for f6c5e398. Neither observation qualifies the later SHA profile candidate or establishes the full hosted gate budget.

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

# Telemetry success paths honor the shared JSON report envelope

## Notes

- **2026-09-30 21:45 UTC, codex:** Related to OW-WAR-0100 and the closure audit at fe3e0b6c. The agreed seam is the existing public war telemetry --json contract. Repair successful command envelopes only; retain baseline schema/metric names, exact verification comparison, refusal behavior, human output and all signed records. This ticket is unverified work, not a Warrant amendment or resolution.
- **2026-09-30 21:59 UTC, codex:** Targeted CLI controls pass on Rust 1.97.1: record, exact unchanged verification, attachment, byte drift refusal with artifact unchanged, missing-reviewer refusal, human verification/attachment text. Shared bare errors retain their pre-existing command=error and envelope exit_code=2 while process status is 1; that mismatch is outside this successful-output repair. Full repository gate remains pending.
- **2026-09-30 22:10 UTC, codex:** Integration check now passes after compiler regeneration: 2001 pass, 352 warn, 0 errors; live status JSON equals compiled corpus projection. All 5699 authored Warrant/SAS/ADR/authority files unchanged. The bare-error process/envelope exit-code distinction is intentional and covered by the existing json_envelope test, rather than a new defect. Full repository gate still pending; no qualification claimed.

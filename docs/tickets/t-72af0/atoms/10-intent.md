# Recover after context preparation is unavailable before worker launch

A completed context-preparation response can be UNKNOWN without an implementation
worker ever starting. Keep its work blocked and its exact diagnostic, but record
execution as stopped so a repaired context source can be retried. Preparation
timeouts, interrupted workers and malformed worker results keep conservative
writer fencing. No authority, verification disposition or release gate changes.

Scope: `apps/openwarrant-web/execution.py` and the real HTTP/SDK/Git control in
`apps/openwarrant-web/test_runtime_recovery.py`; within unsigned OW-WAR-0149.

Observed red: permission-denied fixture source produced `workspace.unavailable`,
no worker marker, blocked work and unknown execution; stopped-state assertion
failed. Targeted green: retry after restored permissions plus existing unknown
writer and changed-contract refusals, 3 tests passed in 6.823 seconds. Synthetic
worker only; no paid model call, native-provider qualification or human assurance.

## Notes

- **2026-10-11 12:18 UTC, codex:** Full local reference web suite completed: 232 tests, 123.288 seconds, OK. Includes real SDK/Git/HTTP unavailable-context remediation and existing timeout, interrupted writer, stale snapshot, checkpoint, repair and verifier controls. Actual admin compile returned 0 with no diagnostics and seven writes. Hosted checks and main merge remain pending; this is performer evidence only.

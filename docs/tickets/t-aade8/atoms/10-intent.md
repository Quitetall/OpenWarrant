# Diagnose intermittent draft checkpoint startup timeout in integration suite

Observed document_draft::concurrent_writer_refuses_and_lock_releases_when_process_ends initial checkpoint timeout at document_draft.rs:167 in the capture-import integration replay. Exact replay passed (0.58 s), unchanged full-suite replay passed 280 tests. Cause UNKNOWN; no deadline or assertion changed. Raw failure and replays: docs/warrants/OW-WAR-0149/implementation/capture-import-20261013/. Do not claim a repair from subsequent passes.

## Notes

- **2026-10-13 23:52 UTC, agent://codex:** 2026-10-13: Instrumented failure observed a live child in jbd2_log_wait_commit at missing initial checkpoint, then reaped signal 9; output empty. Other owned children observed in rq_qos_wait, but no syscall trace or underlying storage diagnosis. Source syncs before publishing checkpoint. Exact replay 0.11s and full module replay pass; these do not repair or explain every historical timeout. Keep original i-73f6 open. Retained proof: evidence/20261013.

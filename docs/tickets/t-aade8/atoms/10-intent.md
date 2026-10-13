# Diagnose intermittent draft checkpoint startup timeout in integration suite

Observed document_draft::concurrent_writer_refuses_and_lock_releases_when_process_ends initial checkpoint timeout at document_draft.rs:167 in the capture-import integration replay. Exact replay passed (0.58 s), unchanged full-suite replay passed 280 tests. Cause UNKNOWN; no deadline or assertion changed. Raw failure and replays: docs/warrants/OW-WAR-0149/implementation/capture-import-20261013/. Do not claim a repair from subsequent passes.

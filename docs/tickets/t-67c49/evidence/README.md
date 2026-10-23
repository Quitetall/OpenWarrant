# HTTP request reader — 2026-10-23

CI40011093425 received400 instead of408 in the unchanged original timeout test.
Its exact syscall result was not logged and remains UNKNOWN. The full failed log
is retained under the durable ow81-evidence-20261023 run archive.

Deterministic controls reproduce two code gaps: Interrupted became400 before
headers/body completed, and a read returning after the absolute deadline could
become400 or complete a late request. Both controls went red before their fixes.
The reader now retries Interrupted and rechecks the same deadline after reads.
Early EOF remains400; size limits and the original408 assertion remain unchanged.

All14 web UI tests and550 CLI library tests pass, including the original socket
test. Strict all-target/schema Clippy and formatting pass. No temporary debug
logging was added. These are performer checks; protected full CI must rerun and
no independent disposition or human acceptance is claimed.

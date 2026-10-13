# Special backup nodes — performer observations

Continued on 2026-10-13 from main 53fe9063. A disposable FIFO in the second
backup reproduced destructive partial recovery: the first current root changed
from `current response` to `older response`, then the process blocked copying
the FIFO. A five-second bounded reproduction fails with that exact byte change.
An earlier unbounded inner command was stopped by its confirmed PID; a first
incorrect exact-name filter ran zero tests and supplies no test evidence.

The whole-backup preflight now rejects every non-directory entry that is not a
regular file, before the first directory is removed. Existing link checks still
apply. The public CLI regression uses a portable five-second child deadline,
checks unchanged earlier and later roots and marker bytes, and requires the
named recovery refusal. A registered FIFO control also requires the nonregular
reason and unchanged records. No live repository recovery or signing occurred.

Three recovery CLI regressions pass. Registered recovery/idempotency controls:
36 PASS, zero FAIL, including valid crash recovery and successful/replayed acts.
CLI all-target Clippy with warnings denied, formatting and shell syntax pass.
Full CI remains required before merge. This closes the special-node defect,
not cryptographic snapshot authentication, concurrent-writer isolation, later
signed-record preservation, power-loss qualification or parent Warrant assurance.

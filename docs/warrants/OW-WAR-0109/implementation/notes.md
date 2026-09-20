# OW109 implementation notes

Availability observation primitive implemented; not connected to live dispatch.
Three subprocess tests cover distinct availability states, malformed/stale/extra
fields, duplicate fields, oversized output, deep nesting, nonzero exit and timeout.
No model calls. No independent verification or assurance claim.

Next: durable enqueue/cancel/consume history; consume before launch and never retry
an uncertain consumed request; associate retained attempt identity; shared admission
recheck; authenticated API and browser integration; full workflow and gate checks.

Generated check after Warrant creation: 954 pass, 88 warnings, zero errors.

Durable queue core now records exact source/configuration requests, immutable
consumption before dispatch and durable queue-to-attempt bindings. Ambiguous
consumed requests never replay; restart can recover an exact retained attempt.
Cancellation applies only before consumption. Polling rotates eligible requests
and performs at most one bounded availability probe per call.

Queue tests cover real SDK/Git/process execution with required checks, persistence,
restart, both launch crash boundaries, cancellation, configuration/admission
refusal and corrupt-history refusal. Public API/browser and background scheduling
remain pending. These are implementation observations, not independent verdicts.

Authenticated queue API, owner availability configuration, background scheduler
and browser enqueue/list/cancel controls are now implemented. Actual HTTP fixtures
observe unavailable -> dispatched -> completed and restart without duplication;
cancelled work remains stopped when availability later changes. Full reference
web suite: 204 tests pass in 69.622 seconds on this pre-main-refresh tree. JavaScript
syntax check passes. Direct browser QA and final full gate remain pending.

One standalone queue rerun omitted OW_TEST_WAR and failed because this worktree
has no target binary; rerun with the explicit existing built SDK passed seven tests.
This environment failure was not treated as product evidence or hidden by a skip.

Post-main merge full suite: 207 tests passed in 68.550 seconds at 5456602.
Direct browser observation on a disposable synthetic fixture confirmed saved
revision enqueue, waiting-for-agent, cancellation, requeue, automatic dispatch,
one completed stopped attempt and unverified labeling. Optional board/hotline/
verifier services were absent in this tiny fixture; their qualification is not
claimed. Browser polling replaced unchanged controls; preserve the unchanged
queue DOM to avoid losing focus/click targets. Syntax checked after that fix.

Additional real-executor admission audit passed five refusal cases: verified-start,
unknown cost with mandatory cap, incomplete dependency, unknown previous writer and
source changed after enqueue. Every case remains queued/blocked, creates no new
attempt and never calls the availability probe. Eight queue tests passed overall.
Follow-up browser observation confirms keyboard focus retained across unchanged
polling and Enter cancellation succeeds.

Final scope audit identified missing explicit report pointers on queue entries.
Added deterministic attempt_path/report_path fields and direct Queue work report
button. Real HTTP test reads both returned paths and observes WORK_DONE only after
actual successful execution. Both HTTP cases pass. The earlier b908185 full gate
does not cover this later view change; fresh hosted gate remains required.

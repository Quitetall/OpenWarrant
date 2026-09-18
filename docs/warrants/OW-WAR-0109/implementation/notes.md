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

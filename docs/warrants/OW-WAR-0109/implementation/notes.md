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

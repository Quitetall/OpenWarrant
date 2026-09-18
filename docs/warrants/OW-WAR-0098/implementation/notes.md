# OW98 implementation checkpoint

Reference workflow exposes authenticated `POST /api/admission` and a browser
check action. Preview and Start share exact-subject, configured-start, cost,
attempt, dependency and Git-base checks under one lock. Start rechecks live state.
Preview never grants dispatch or qualification; worktree identity, writer claim
and launch remain action-time checks. Legacy CLI convergence is still outstanding.

Validation: 40 web HTTP/process/Git tests passed, including three new admission
tests. Corpus generated check passed. Spec source review passed with one UI nit:
preview now uses the same unsaved-edit and history guards as Start. Standards
source review passed. Neither reviewer independently ran runtime qualification.

Full repository gate passed: 14/14 steps and 308 plants on c1e1f61. LAMU commit review returned PASS WITH NITS; false positives and addressed UI duplication are recorded in evidence/review.md. Final evidence commit review remains recorded in the session.

## Policy-bound completion follow-up

Real HTTP regression reproduced a ready dependent after adding a new required
check to its dependency's configured policy. Existing admission used only source
identity and completed/stopped labels. The same saved result was still counted
by the progress projection despite the stronger current policy.

Admission and progress now share exact source/policy eligibility plus established
required checks. Historical attempts remain unchanged. Public HTTP tests exercise
unchanged-policy readiness, changed-check refusal, Start/preview parity, and stale
progress exclusion. Projection tests also cover base/dependency/verified-start
changes and explicit historical-result labels. Full package tests pass; exact-head
hosted gate and independent qualification are still pending. Prior progress record
and red/green observations are retained under `policy-completion/`.

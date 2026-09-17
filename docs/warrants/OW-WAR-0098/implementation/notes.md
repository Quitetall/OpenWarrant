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

Full repository gate and commit review remain pending at this checkpoint.

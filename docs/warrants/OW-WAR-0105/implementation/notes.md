# Agent drafting candidate

Implements configured process drafting through authenticated HTTP and the reference
workbench. The adapter receives the war-spec method, bounded prompt and pinned
context. Actual SDK authoring validates the returned fields. Results remain unsaved;
the user imports into an editable draft and saves through the existing revision API.

Retained initial/final records support identical-request replay without new launch.
Interrupted requests remain unknown; unknown workers block replacement. Shared
bounded process transport preserves execution's existing timeout/output semantics.
Unknown cost refuses a cap; free is an owner assertion. No actual model was called.

Validation: 71 package tests pass. Separate logs retain the original execution
suite, focused drafting and interruption tests. JavaScript syntax check passes.
Full repository integration gate is pending. No independent assurance is claimed.

Browser observation used a task-owned loopback server (port 33089), disposable
state, synthetic adapter and public fixture token. Observed:

- Prompt submission displayed a running then ready retained attempt.
- Expanded attempt showed human-first Markdown and its RC.3 metadata footer.
- Import while the editor contained an unsaved title refused and retained that title.
- Explicit New draft followed by import populated all four fields without saving.
- Save produced revision 1 with the request's UUID and unverified status.
- No execution harness was configured; no implementation started.
- Temporary tab closed; task-owned server stopped after observation.

The browser loaded the prior introductory sentence; its misleading claim that
'drafting does not run an agent' was corrected in source to say it does not
implement code. This wording-only adjustment was not reloaded in that walkthrough.

The war-spec skill's stale statement that RC.3 authoring was future work now points
to shipped SDK commands. OW68's prior D-005 manifest and skill bytes remain under
`attempts/agent-drafting-20260918/`; the new candidate is unverified.

Remaining: full gate and exact-head integration, source inspection, actual
local/cloud adapters and their model-quality/cost/isolation evidence. Synthetic
fixtures establish transport behavior only. This does not close Phase 3 or OW42.

Integration with policy-bound completion passes 73 package tests. The first run
failed at service startup because OW_TEST_WAR was unset in this worktree; the
retained retry sets the built binary path. Both logs remain available.

Missing adapter command now has a direct HTTP refusal test: retained failed attempt,
no source saved, identical replay returns the same record without dispatch.
The first full gate at f458ac9 ended with three unavailable steps because the
standalone clone lacked target/debug/war while CARGO_TARGET_DIR selected the shared
build output. Retained log records this failure. The clone-local target symlink
restores the expected binary location; full rerun is pending.

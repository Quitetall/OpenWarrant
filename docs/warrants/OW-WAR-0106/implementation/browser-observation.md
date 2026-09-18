# Hotline browser observation

Disposable loopback fixture at port 35681 used the actual reference server, SDK,
Git worktree and synthetic harness. Public fixture credentials authorize only this
disposable test. No real model or human signature was involved.

Observed in the in-app browser:

1. Unlock showed one technical question, exact source/checkpoint/question digests,
   STAGE-001, and configured fixture-adviser (ai). Original attempt was blocked
   with execution stopped. No completion signal appeared.
2. Wrong responder credential returned authentication refusal; answer/evidence
   text stayed available and credential input cleared.
3. Valid credential recorded the answer and evidence. The UI explicitly said work
   remained paused. No automatic execution followed the answer.
4. Resume created attempt 618bdb24-a705-46e9-ab95-a49ec8294496; question view named
   the resumed attempt and removed the resume action.
5. Refresh showed completed/stopped at result
   549898a4ced1dfbe882546a52229bde08ec6f5ac. The original blocked attempt remained.
6. Work report displayed FIXTURE_DONE only for the completed result, identified
   current-policy eligibility and unverified qualification, and offered offline HTML.
7. Lock returned to the empty session-token screen and removed hotline contents.

The minimal fixture repository intentionally lacks the legacy project corpus;
board/project controls reported unavailable. They were not part of this scenario.
The temporary browser tab was closed. Task-owned PID 3327032 was stopped and its
process handle returned exit 143. Existing user services/tabs were untouched.
JavaScript syntax check and 38 execution-package tests pass. Offline file rendering,
automatic adviser routing and real-provider qualification are not established here.

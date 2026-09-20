# Stage selection browser observation

Disposable loopback fixture at port 33685; public test credential, synthetic
harness, isolated Git repository. No real model, human approval or production
sandbox qualification. User tabs and services were not changed.

Observed through browser UI:

1. Selected saved Warrant and loaded stages. API ready, UI blocked, zero attempts.
2. Selected UI and pressed Start: prerequisite refusal, no successful launch.
3. Selected API and pressed Start. Attempt b7fce04a-d91f-48fd-996b-6d5b9204a833
   stopped with Warrant in-progress. Reloaded stages: API completed, UI ready.
4. Selected UI and pressed Start. Attempt 350bbbf0-f728-484f-9e85-fcbb28b3d09a
   completed at d84056172cdfeffb46b106963a44907a2759f648.
5. Opened final work report. Displayed FIXTURE_DONE, completed, stopped,
   qualification unverified, current policy eligible yes. Offline download link
   present; this observation does not claim downloaded-file rendering.
6. Locked session: work details cleared. Closed only task-created fixture tab.

Minimal fixture does not contain legacy corpus or configured drafter/hotline.
Their unavailable panels were expected and outside this stage-selection test.

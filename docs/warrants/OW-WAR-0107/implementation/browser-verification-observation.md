# Browser verification observation

Observed 2026-09-18 against code `1417cee` in a disposable local service on
`127.0.0.1:44789`. The fixture created a real SDK Warrant, ran the configured
synthetic implementation harness to a committed result, then exposed the actual
Store, Executor and Verification service. No production model or user key was used.
The fixture machine issuer asserted synthetic protections; this is not evidence
that a deployment harness enforces isolation or independent model behavior.

Observed through the Codex in-app browser, temporary tab 9:

1. Unlocked the fixture session. Completed execution appeared with its exact source
   and candidate revision and a Prepare independent verification button.
2. Clicked Prepare. Panel displayed prepared, unknown result, evidence not_received,
   and qualification unverified. No process started.
3. Generated a disposable machine-signed receipt bound to the prepared job and
   admission basis. Uploaded its JSON through the browser file chooser.
4. Clicked Start independent verification. Panel displayed running with retained
   evidence and unverified qualification.
5. Refreshed. Panel displayed finished, PASS, retained evidence and unverified
   qualification. The synthetic verifier summary appeared. The API observation is
   retained in `browser-verification-result.json`.
6. Sequestered the fixture receipt without changing job history. Refreshed. Panel
   displayed UNKNOWN and evidence unavailable, retaining historical result text.
   API observation: `browser-verification-missing-evidence.json`.
7. Locked session. Verification panel disappeared and lock screen returned.

Restored the fixture receipt, closed temporary tab and stopped only the recorded
fixture PID. Process session 90187 ended with exit 130 from deliberate SIGINT;
service cleanup ran. Existing user tabs and services were not stopped.

The empty fixture repository had no legacy project board and no configured
drafting/hotline provider; their separate panels reported unavailable as expected.
No claim covers those panels here. Request download, automatic polling timing,
repair/rebuttal, aggregate budgets and deployment qualification remain outside this
observation. Reproduction script: `browser-verification-fixture.py`, run with the
app directory on PYTHONPATH and OW_TEST_WAR pointing to a built SDK binary.

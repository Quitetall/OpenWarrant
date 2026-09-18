# OW-WAR-0107 implementation audit

This is performer evidence, not independent verification or Warrant resolution.
The audited code revision is `4a2682f2274000de20b9e22f373666192f53b3e4`.
Read the authored work order and assurance atoms for the actual scope.

| Work-order item | Current implementation and evidence | Limits / remaining proof |
| --- | --- | --- |
| 1. Configuration and exact contracts | `verification.py`, `verifier_policy.py`, `verifier_snapshot.py`; protocol, policy and snapshot refusal tests | Operator configuration and protected storage are trust inputs, not inferred isolation. |
| 2. Independent dispatch | Separate Git workspace, configured identities, signed capability receipt, durable claim and process controller; workspace, attestation, dispatch, controller and HTTP tests | Synthetic harnesses establish protocol controls. A signature cannot establish that its issuer truthfully enforced a sandbox. |
| 3. Retained observations and stale rejection | PASS/FAIL/UNKNOWN records, protected checks, current-basis recheck, missing-receipt UNKNOWN view; controller/service and browser evidence | Historical verdict and current effective verdict remain distinct. No common assurance mark is awarded. |
| 4. Bounded repair loop | Existing writer/worktree controls, aggregate accounting, selected/default cycle limit, staged checks, durable loop scheduler and receipt inbox; repair, budget, scheduler tests and browser-loop artifacts | Hard caps refuse unknown cost; no paid reservation interface is invented. External harness receipt generation remains provider-owned. |
| 5. Rebuttal and human decisions | Exact prior observation in rechecks, authorized responder decisions, current-authority enforcement, sibling-dispatch refusal, loop traversal of rechecks and hotline resume | Fixture human credentials prove authentication behavior, not actual human presence. No fixture act accepts or signs repository work. |
| 6. API/browser and reports | JSON inventories and repair/dispute/loop views; browser repair, rebuttal, decision, recheck, enable/pause/resume controls; retained browser artifacts | A subsequent report supplement integrates verification, repair and dispute history into offline reports; its tests are recorded separately from this audited revision. Browser stale-response/refusal coverage is narrower than backend coverage. |
| 7. End-to-end tests and gates | Real local Git/process/HTTP tests and disposable browser sessions; latest web suite: 189 tests passed in 61.960 seconds | Full Rust gate passed on this revision with Rust 1.97.1: 14 steps, 308 plants, zero failures. CI and final release acceptance are not established by these local checks. |

## Evidence rules

- Tests and browser fixtures do not satisfy an independent verifier's verdict.
- The 88 warnings in the generated corpus check remain warnings; zero errors
  does not resolve legacy Warrants or accept SAS RC.3.
- No model review was requested or run. The owner's suspension of mandatory
  model reviews remains in effect; independent release/human gates remain intact.
- This Warrant is still open. Do not infer completion from this audit or from
  implementing the API endpoints.

## Report supplement and exact-head CI observation

At `843901d6fa43ad0528017c4e6fe784886d4cb998`, work-order item 6 also
includes `verifier_reporting.py`: deterministic v2 report supplements retain
verification observations, repair records and dispute decisions. The original
execution snapshot digest remains distinct from the verification snapshot digest.
The 192-test web suite covers real failed-candidate repair followed by PASS,
escaped findings, repeated reads, minimal output, missing-receipt UNKNOWN while
preserving historical PASS, no-job prototypes and the authenticated HTTP route.
This extends the earlier 189-test audit; it does not imply independent assurance.

Hosted web workflow run `35341196200` passed at this head. The actual Bonsai
artifact from run `35341196754` is retained as `pr118-bonsai-843901d.json`.
Its head, Git tree, scope-file digest and policy digest were checked against this
checkout. Verdict is PASS with no scope or architecture findings. This replaces
neither human acceptance nor the Rust gate. The first PR run skipped Bonsai due
to missing PR-body metadata and is not counted as Bonsai evidence.

Rust gate job `105587307610` in run `35341196754` was still in progress at this
observation. No final-head gate success or implementation closeout is claimed.

## Hosted gate correction

Run 35341779967 later failed `the_corpus_projection_is_the_result_payload_verbatim`:
the machine scope gained the required threat-model path without regenerating
Warrant and corpus projections. The scope record is correct; projections were
regenerated through `war compile` rather
than edited by hand. `war check --generated` then reported 942 pass, 88 warnings,
zero errors. A new hosted gate is required; the failed run remains failed.

The six JSON-envelope integration tests then passed, including the exact test
that failed in CI. Scope-digest changes in generated WAR.md confirm this cause;
progress JSON alone did not cause the drift.

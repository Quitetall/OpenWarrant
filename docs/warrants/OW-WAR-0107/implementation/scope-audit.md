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
| 6. API/browser and reports | JSON inventories and repair/dispute/loop views; browser repair, rebuttal, decision, recheck, enable/pause/resume controls; retained browser artifacts | Work-stop report integration needs a final review: execution reports and verification inventories are separate surfaces. Browser stale-response/refusal coverage is narrower than backend coverage. |
| 7. End-to-end tests and gates | Real local Git/process/HTTP tests and disposable browser sessions; latest web suite: 189 tests passed in 61.960 seconds | Full Rust gate is running in a standalone clone. CI and final release acceptance are not established by these local checks. |

## Evidence rules

- Tests and browser fixtures do not satisfy an independent verifier's verdict.
- The 88 warnings in the generated corpus check remain warnings; zero errors
  does not resolve legacy Warrants or accept SAS RC.3.
- No model review was requested or run. The owner's suspension of mandatory
  model reviews remains in effect; independent release/human gates remain intact.
- This Warrant is still open. Do not infer completion from this audit or from
  implementing the API endpoints.

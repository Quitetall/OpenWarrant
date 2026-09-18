# OW107 implementation notes

Drafted from candidate SAS RC.3 sections 14 and 15 and existing execution contracts.
OW106 integration is a publication prerequisite. Draft check: 15 passes, one
warning, zero errors. Generated corpus: 942 passes, 88 warnings, zero errors.
These observations validate records only. No verification runtime is implemented
by this draft. Planned first seam: configured verifier identity/capabilities and
exact candidate request/result validation, then isolated process/workspace tests.

## Verifier contract candidate

Added bounded request/result validation for controller-owned identities and exact
source bytes, policy digest, candidate commit and check commands. Results bind the
full request digest. Distinct performer/verifier names are required, but naming
alone does not establish independent context, workspace or protected execution.
Those checks belong to the forthcoming runtime boundary. No result accepts an
actor or qualification field. PASS cannot retain unresolved findings; FAIL needs
an observed violation; UNKNOWN stays distinct and cannot instruct automatic repair.
Four refusal-oriented contract tests pass. No harness was launched by these tests.

## Separate workspace and bounded process seam

Verifier workspace is a fresh Git repository populated by exact-commit fetch,
with separate object/ref storage. It excludes performer uncommitted scratch.
Existing, symlink and performer-nested destinations refuse without overwrite.
Required checks run on the exact unchanged candidate before the configured
verifier command. A failed check is retained and skips verifier launch. Candidate
mutation, stale output and timeout remain UNKNOWN, never PASS. Eleven distinct
contract/workspace/process tests pass, including real Git and subprocess refusal
observations. This seam is not yet exposed as a server action and is not a sandbox.

Next: protected configuration/admission, persistent verification attempts and
public HTTP integration, then bounded repair/rebuttal/escalation and browser QA.
No real model calls, independent qualification or human acceptance occurred.

## Verifier policy and advisory admission

Added strict configured performer/verifier identities and commands, finite time
limits and explicit cost policy. Admission matches retained performer command and
current completed candidate policy. Unknown cost refuses a mandatory hard cap.
Configuration does not accept capability flags as proof. Missing authenticated
protection evidence is UNKNOWN; stale, failed or unavailable observations cannot
produce ready. The evaluator takes trusted observations only at an internal caller
boundary; no browser route supplies them. Ready is advisory and grants no dispatch.

Fourteen distinct verifier contract/workspace/process/policy tests pass. Fixture
protection observations exercise evaluation, not real harness authentication.
The next controller must authenticate evidence outside performer/verifier output
before passing it here; that integration and durable jobs remain unfinished.

## Machine-authenticated protection observations

Added SSH Ed25519 signature verification over exact received receipt bytes using
an operator-pinned public key/principal and dedicated harness-protection namespace.
Evidence binds admission basis and dispatch nonce, with a maximum five-minute
validity window. Signed FAIL/UNKNOWN stay FAIL/UNKNOWN; injected qualification
fields refuse. Signature verification does not make an issuer honest or provide
human presence. Harness key custody, protected app configuration and actual
protection enforcement remain deployment obligations. Controller must reserve
nonces and prevent duplicate dispatch; durable job integration remains next.

Seventeen distinct verifier tests pass, including real ssh-keygen signatures from
disposable machine keys. Cases refuse changed bytes, different key, different
namespace, wrong basis/nonce, expired/future/overlong validity and extra authority
fields. No user keys, human signatures or real-model calls used.

## Durable verification claims

Added an append-only three-record store: prepared request and admission basis,
consumed dispatch claim with protection receipt digest, then final observation.
Atomic nonreplacement publication permits one controller to consume a claim.
Restart cannot relaunch a consumed claim; without a live process observation it
shows UNKNOWN. Hash-linked records reject missing predecessors and changed bytes.
These hashes detect corruption; they do not authenticate storage against an actor
with write access. The harness must protect storage and the controller must check
authenticated admission before consuming a claim. The store itself grants neither
dispatch authority nor qualification.

Twenty-two verifier tests pass, including sixteen concurrent claim consumers with
exactly one winner, restart/replay, stale candidate reuse, altered history, symlink
refusal and missing/changed/noninteger check evidence. PASS requires an exact bound
verifier result and all protected checks. FAIL requires an observed finding or
failed protected check. The controller, HTTP actions, bounded repair and rebuttal
loop remain unfinished; this is implementation progress, not Warrant completion.

## Authenticated claim consumption

Connected machine signature authentication, current candidate admission and durable
claim consumption. The protected request must match current Warrant identity,
candidate, source, execution policy, configured actors and exact checks. Changed
verifier configuration invalidates the prepared basis. Signed unknown or failed
protection observations cannot consume a claim. Exact signed bytes, signature and
issuer public identity are retained before consumption; a replay returns no launch
permission. Callers must hold the execution lock and supply fresh protected inputs.
The HTTP controller and actual dispatch integration remain unfinished.

Twenty-six verifier tests pass. Four new cases use disposable machine keys and
exercise exact signed acceptance, replay, signed UNKNOWN/FAIL, byte tampering,
wrong nonce, changed candidate/checks/actor/Warrant/configuration. These are bounded
protocol observations, not proof of production sandboxing or independent review.

## Process lifecycle integration

Added a synchronous controller seam connecting protected snapshot reads, exact
candidate cleanliness, signed claim consumption, isolated verifier execution and
durable final observations. It copies the initial snapshot and compares fresh
protected inputs after execution. A changed candidate or configuration yields
UNKNOWN while retaining the original verifier observation. Replayed consumed jobs
return retained state without launching another process. Callers must provide the
execution lock and a trusted snapshot reader that refuses active or unknown writers.
No public request can supply that reader; server route integration is still pending.

Thirty verifier tests pass, including a real Git/process/signature lifecycle,
replay after replacing the verifier program with a failing one, dirty-candidate
refusal before consumption, changed policy after PASS, and a synthetic verifier
that changes the performer workspace. The latter demonstrates stale-result
refusal, not sandbox enforcement. HTTP/browser integration, aggregate budgets,
repair/rebuttal and production protection qualification remain open.

## Protected executor snapshot reader

Added the concrete internal snapshot reader for executor records and owner-configured
verifier/issuer files. It rereads verifier configuration on each call and returns a
detached value. Active or unknown Warrant/dependency writers, missing dependency
completion, changed source, stale policy, changed performer command and unrelated
worktree paths refuse. File reads use the server's bounded non-symlink reader.
The caller must hold the executor lock. Harness enforcement still owns protection
of configuration and records against agents with filesystem access.

Thirty-four verifier tests pass. New snapshot tests use retained-state fixtures,
not a public HTTP request. Server wiring, dispatch inventory, repair/rebuttal and
browser qualification remain unfinished.

## Preparation API and retained inventory

Added opt-in service configuration and authenticated preparation/list/get routes.
Clients submit exact execution and verification UUIDs. The service derives source,
actors, checks and candidate from protected current state, persists an immutable
binding and prepares the claim. Reusing an ID for changed work refuses. Preparation
does not start a process or grant dispatch. Public start and browser controls remain
next, followed by aggregate budgets and repair/rebuttal.

Full Python web regression: 146 tests passed in 45.876 seconds using
`OW_TEST_WAR=/mnt/4tb/tmp/ow-question-integrity/target/debug/war`. New HTTP tests
cover authentication, unsupported authority fields, unknown writers, exact replay,
changed-candidate refusal and service reconstruction over retained jobs. These use
executor state fixtures; the full production execution-to-verification HTTP flow
is not established yet. Evidence: `implementation/web-regression.log`.

## Explicit background dispatch API

Added authenticated `POST /api/verification/<id>/start` with bounded base64
receipt/signature fields. Signature and current candidate checks happen before
claim consumption. The service then schedules the isolated verifier in a background
thread and exposes live/retained state through existing job routes. Replayed claims
never launch again. Any prior running or uncertain verifier blocks a new dispatch.
Failure to persist final state leaves the consumed claim UNKNOWN after its worker
ends; it does not free that claim for retry.

Full web regression: 149 tests passed in 47.303 seconds. New public HTTP cases
exercise disposable machine signatures, real Git/check/verifier subprocesses,
durable PASS without qualification, replay after service reconstruction, forged
receipt refusal before consumption and interrupted-claim refusal. Executor records
are fixture inputs, so production execution-to-verification and actual sandbox
protection remain unqualified. Browser controls, aggregate budgets and bounded
repair/rebuttal remain open. Evidence: `implementation/dispatch-web-regression.log`.

## Missing protection evidence

A public HTTP regression exposed missing evidence-loss reporting after deleting a
protection receipt from an otherwise completed PASS job. Job views now preserve
the historical observation but report `evidence_state: unavailable` and
`effective_verdict: unknown` if its retained protection receipt is missing or has
the wrong digest. Exact receipt restoration restores the evidence view; corrupt
replacement remains UNKNOWN. New dispatch refuses when prior evidence is missing.
No authority record or original observation is deleted or rewritten.

Forty verifier tests pass after the fix, including HTTP deletion, exact restoration
and corruption cases. Evidence: `implementation/evidence-retention-tests.log`.
The earlier full web run remains 149 passes; this change ran the focused verifier
suite. Browser controls and repair/budget work remain open.

## Browser control candidate

Added preparation buttons on completed stopped runs and a verification panel with
exact request/evidence details, request download, bounded signed receipt file upload,
explicit start, readable findings and effective evidence status. Running jobs refresh
every two seconds. Lock clears the panel and polling; late verification responses
check session identity before updating the view. Text uses DOM textContent.

Extracted script passes `node --check`; `git diff --check` passes. Real browser
interaction is not yet observed, so UI qualification remains pending. This change
does not complete OW107 or its remaining aggregate budget/repair/rebuttal scope.

## Browser observation

Real browser preparation, signed receipt upload, explicit start, running-to-PASS
display, evidence-loss UNKNOWN and lock clearing were observed against a disposable
service using the real SDK, Git and execution controller. Full bounded evidence and
limits are in `browser-verification-observation.md`. This establishes that specific
UI path with synthetic processes; it does not qualify external harness protection,
model independence or the remaining repair/budget scope.

## Aggregate verifier admission budget

Verifier dispatch now accounts for retained execution and verification attempts
across IDs for the same Warrant. The execution time ceiling limits total active
time; the verifier timeout also bounds its individual run. Both configured spend
caps apply, with the smaller cap controlling. Missing/nonfinite/negative usage,
uncertain execution and unknown cost under a hard cap refuse before consumption.
Prepared jobs do not spend budget. Completed verifier observations now retain
measured active time and explicit zero/unknown cost according to configured mode.

Forty-five verifier tests pass, including aggregate exhaustion across changed IDs,
unknown/missing accounting, either hard cost cap, and HTTP refusal before creating
a verifier workspace. Evidence: `implementation/budget-tests.log`. No metered paid
backend or reliable paid reservation interface is implemented. Repair dispatch
still needs this shared accounting when the repair loop is connected.

## Bounded repair planning

Added retained-evidence repair planning and its authenticated read-only API.
Repairable findings and failed protected checks can produce an advisory ready plan.
UNKNOWN/nonrepairable findings escalate; missing evidence or uncertain verifier
execution block. Explicit cycle limits override the fallback of three, including
zero. Repair history counts initial repair dispatches across verification IDs,
not hotline resumptions or unrelated Warrants. A prior repair for the same result
returns that attempt instead of proposing another dispatch. No performer can clear
the independent finding through this API.

Thirteen focused repair/service tests pass, including default/selected/exhausted
limits, idempotency, unknown evidence, failed-check repair and HTTP PASS preview.
Evidence: `implementation/repair-plan-tests.log`. This plan is based on retained
observations and grants no dispatch. Actual repair execution, fresh-state admission,
automatic re-verification and rebuttal/escalation routing remain open.

## Explicit repair execution

Connected verifier-directed repair to the existing execution controller and
authenticated POST repair route. Before dispatch, the service rechecks exact
candidate/source/policy/verifier basis, stopped writers, retained evidence, cycle
limits and shared accounting. Existing worktree ownership and serialized writer
claims remain in force. Repair context carries the original observation digest,
findings and failed checks in execution request v4. Repeated requests return the
same attempt. Hotline resume retains repair lineage without consuming a new cycle.
Staged repair reruns all stage checks before whole-Warrant checks; that new staged
repair path still needs its own end-to-end scenario.

Real SDK/Git/process test observed independent FAIL, repair in the same worktree,
new committed candidate and a new independently dispatched verifier PASS. The old
FAIL remained unchanged and no qualification was granted. Separate cases refused
zero repair limit, exhausted time, replacement-check injection and stale candidate
before creating a repair attempt. Full web regression: 162 tests passed in 49.266
seconds (`implementation/repair-web-regression.log`). An earlier focused invocation
also ran 46 existing execution tests because a TestCase was imported into discovery;
the import now uses a module alias to avoid duplicate discovery.

Automatic repair/reverification scheduling, rebuttal/escalation and dedicated
staged-repair/hotline-resume evidence remain open. This is not OW107 completion.

## Staged repair and hotline recovery evidence

Expanded real SDK/Git/process fixtures to staged Warrants. Successful repair reran
both API and UI stage checks on the new commit, then passed a fresh independent
verifier. A planted regression in the previously completed API stage produced a
stopped failed repair; verification preparation refused that incomplete result.

A repair that emitted a technical hotline question resumed after a configured AI
responder's fixture answer. The resumed execution retained its verification-repair
binding, consumed the remaining time allowance and counted one repair cycle across
the initial and resumed attempts. Its new candidate passed fresh verification.
Five focused execution scenarios passed in 2.806 seconds; evidence is retained in
`implementation/staged-repair-tests.log`. These use synthetic processes and do not
qualify model quality or harness isolation. Automatic orchestration and
rebuttal/escalation remain unfinished.

## Evidence-backed rebuttal and independent recheck

Added verification request v2 for exact prior observations plus bounded challenge
arguments, evidence and existing finding IDs. The service prepares a separate
immutable recheck job; it does not edit FAIL or launch implicitly. Fresh signed
protection evidence, shared budget and independent execution remain required.
One recheck binds each challenged result. Unresolved recheck sets a human-review
flag and blocks repair through both parent and recheck, preventing a fallback to
the old repair route from bypassing the disagreement.

Seven real SDK/Git/process repair/recheck scenarios pass in 3.634 seconds, including
retained original FAIL, independently repeated FAIL, visible human-review need,
repeat challenge/repair refusal and missing evidence/invented finding/authority
field refusal. Evidence: `implementation/rebuttal-tests.log`. The new HTTP route
uses existing authenticated JSON handling; dedicated HTTP/browser rebuttal QA,
authorized human settlement and automatic orchestration remain unfinished.

## Authorized dispute decision records

Added exact-basis human dispute questions, authenticated decision records and
GET/POST dispute routes. The existing responder configuration supplies identity,
human kind and per-Warrant governing scope. Caller fields cannot select actor or
award acceptance. AI responders, wrong scope/credential, stale basis, unsupported
acceptance acts and missing evidence refuse. Decisions select repair, verify again,
revise scope or stop and remain immutable; they do not rewrite findings or launch.
Missing responders produce visible waiting_for_authorized_responder state.

Eleven decision and real process scenarios passed in 3.672 seconds. Fixtures prove
decision persistence, exact replay, conflicting-decision refusal and unchanged FAIL
after a decision, using synthetic human-role credentials only. No real human act
or presence is claimed. Evidence: `implementation/decision-tests.log`. Applying
decision effects to dispatch, dedicated HTTP/browser decision QA and automatic
orchestration remain unfinished.

## Applying human repair decisions

A current retained human repair decision now permits repair of an unresolved
recheck through its exact verification identity. The repair record retains the
decision digest. Responder configuration changes prevent decision reuse; scope,
candidate, protected checks, writer state and aggregate budgets still revalidate.
Stop, revise-scope and verify-again decisions refuse repair through this endpoint.
Human repair permission does not turn unknown findings into known defects or
bypass missing evidence and cycle limits. Original FAIL observations remain.

Nineteen focused decision/repair scenarios passed in 4.719 seconds. The full web
suite passed 173 tests in 52.561 seconds using
`OW_TEST_WAR=/mnt/4tb/tmp/ow-question-integrity/target/debug/war python3 -m unittest discover -q`.
Evidence: `implementation/human-repair-web-tests.log`. Tests use synthetic human
credentials, real local Git/process execution and configured protected-receipt
fixtures; they do not prove real human presence or deployment isolation.

Verify-again dispatch, broader stop/scope-change orchestration, dedicated browser
repair/dispute controls, automatic loop orchestration and the full Rust/release
gates remain unfinished. This evidence does not close or qualify OW-WAR-0107.

## Human-directed independent recheck

The reverify API prepares one v3 verification request per retained human
verify-again decision. It carries the exact prior final record and decision.
Preparation and dispatch require current authority and unchanged work basis;
dispatch still needs a fresh signed protection receipt and aggregate budget.
Exact replay returns the existing child. A different child identity refuses.
An unresolved v3 result requires another human decision, without clearing prior
FAIL observations or granting qualification. Revoked authority leaves the claim
prepared and unconsumed. Stop and scope-revision decisions cannot use this route.

Real-process fixtures prove preparation, replay, duplicate refusal, authority
removal before dispatch, restored-authority dispatch and repeated unresolved FAIL.
Contract checks refuse candidate, observation, decision-action and responder-kind
substitution. HTTP tests cover authentication and invalid-identity refusal; a
positive HTTP/browser human recheck ceremony remains untested.

The full web suite passed 173 tests in 52.772 seconds with
`OW_TEST_WAR=/mnt/4tb/tmp/ow-question-integrity/target/debug/war python3 -m unittest discover -q`.
Evidence: `implementation/reverify-web-tests.log`. Synthetic credentials and
processes establish protocol behavior, not human presence or deployment isolation.
Automatic orchestration, browser repair/dispute controls, broader stop/scope-change
handling and full release gates remain unfinished. OW-WAR-0107 remains open.

## Refuse dispute bypass through sibling claims

Inspection found that a generic prepared verification or sibling failed result
could bypass an unresolved human dispute. Verification and repair dispatch now
check unresolved leaf disputes for the same Warrant, source and candidate. Only
the matching current human-directed action may proceed. Completed child checks
advance the chain without deleting prior verdicts. A different source is evaluated
under its own policy; this control does not stop processes outside the service.

Real-process tests plant spare claims before the dispute, attempt sibling repairs,
restart the service and confirm refusal with no claim consumption or new writer.
Stop, revise-scope and verify-again each run in a separate fixture. Repair preview
uses the same dispute check so it cannot advertise a blocked sibling as ready.

The full web suite passed 175 tests in 53.605 seconds before the final preview
alignment. A subsequent focused run covers the final preview and dispatch code;
its exact result is retained in `implementation/dispute-preview-tests.log`.
Full-suite evidence: `implementation/dispute-guard-tests.log`. Both runs use
`OW_TEST_WAR=/mnt/4tb/tmp/ow-question-integrity/target/debug/war` and unittest.
No independent assurance or full release gate is claimed. Browser controls,
automatic orchestration and remaining OW-WAR-0107 scope remain unfinished.

## Browser repair, rebuttal and human-decision controls

The workbench now exposes repair plans, bounded repair dispatch, retained repair
report links, selected-finding rebuttals, configured human decision forms and
human-directed recheck preparation. Each async action checks its original session
and attached card before updating UI. Responder credentials clear on submission.
Decisions display short summaries with expandable exact evidence.

A disposable browser fixture at port 41161 demonstrated:
- ready repair preview, zero of three cycles;
- browser rebuttal with selected F1 and evidence;
- synthetic verify-again decision and v3 recheck preparation;
- synthetic repair decision after repeated FAIL;
- browser repair dispatch, then completed stopped attempt
  dfe9bb84-e010-44b4-80a1-786992875a1d at
  990df3bc9fa62d1f9845ebaf794162e680afd09b;
- retained original candidate ec689f6de879e0c50be37e07bbda46374625d1eb
  and all three FAIL observations;
- already-dispatched repair view with one of three cycles and report link;
- lock clearing all workflow panels, then final compact decision presentation.

Verifier starts used a synthetic machine signature through the HTTP API; this run
did not retest browser receipt upload. Human-role credentials were disposable test
values, not real human acts. The fixture's project board lacked a legacy corpus;
its unavailable state is not evidence about the actual repository board.

Retained artifacts: `browser-repair-jobs.json`, `browser-repair-attempts.json`,
`browser-repair-fixture.py`. JavaScript syntax passed `node --check`; diff whitespace
passed. This HTML-only change did not rerun the unchanged backend suite. Test tab
closed and owned fixture process stopped. Automatic orchestration, refusal/race UI
coverage and full release gates remain open; OW-WAR-0107 is not closed or qualified.

## Durable loop transition driver

Added an internal tick driver over existing verification and repair services.
It derives deterministic claim identities from root/current execution attempts,
waits for signed receipt lookup, dispatches verification and bounded repair, and
follows retained repair/resume lineage. It reuses service authority and budget
checks. A retained PASS is returned as checks_passed only after current policy,
source and clean candidate revalidation; qualification remains false.

Real-process fixtures exercised FAIL -> repair -> PASS with retained history and
no duplicate work after driver restart, absent receipt waiting, forged receipt
refusal, zero repair budget refusal and changed-candidate refusal. A scheduler-loss
fixture consumes the real signed claim without launching its worker; subsequent
ticks retain UNKNOWN and request no new receipt or writer.

The full web suite passed 178 tests in 55.202 seconds before the final state-label
rename and added scheduler-loss test. Four final focused loop tests passed in
1.456 seconds. Both used OW_TEST_WAR=/mnt/4tb/tmp/ow-question-integrity/target/debug/war.
Exact logs: implementation/loop-tests.log and implementation/loop-final-tests.log.

The driver is not yet enabled in the service. Host scheduling, protected receipt
lookup, public API/browser status and real external harness receipt production
remain unfinished. The receipt callback must remain read-only and must not launch
unaccounted paid calls. This work does not establish independent assurance or close
OW-WAR-0107; full release gates and remaining scope still apply.

## Read-only harness receipt inbox

Added a directory-descriptor-based inbox adapter for signed harness receipts.
It reads only exact UUID-named regular files, refuses symlinks/FIFOs/oversize or
unexpected fields, and retains original evidence. Missing receipt returns wait;
invalid receipt refuses. Directory replacement does not redirect the open reader.
No receipt producer or paid provider runs inside the adapter. Signature and
current-claim authentication remain in the existing dispatch service.

Nine inbox and loop tests passed in 1.955 seconds. A real Git/process loop consumed
two atomically published synthetic receipts and completed FAIL -> repair -> PASS,
retaining both receipts and both execution attempts. Separate tests cover
filesystem refusal cases and unchanged loop limits/recovery. Evidence:
implementation/receipt-inbox-tests.log. Synthetic signing proves protocol behavior,
not deployed sandbox enforcement or human acceptance.

Service scheduling, opt-in loop API/browser controls and real harness integration
remain outstanding. This adds the receipt lookup seam; it does not enable an
unattended service or close OW-WAR-0107.

## Opt-in service loop scheduling

Added a background scheduler, append-only enable/pause intent histories and
GET/POST /api/verification-loops. The operator enables the facility with
--verifier-receipts PATH alongside verifier configuration; each completed root
attempt must then be explicitly enrolled. Browser/API callers cannot select
commands or inbox paths. Duplicate requests retain one intent revision. Pause
serializes with ticks and prevents future launches, while existing processes stay
under their original bounds. Enabled histories are read on restart; observations
start empty and consumed/uncertain claims still cannot relaunch.

Tests demonstrate background FAIL -> repair -> PASS, pause across scheduler
restart, HTTP authentication and exact-field refusal, missing-receipt waiting,
registration without immediate dispatch, and corrupt-history refusal. Full web
suite: 188 tests passed in 57.213 seconds using
OW_TEST_WAR=/mnt/4tb/tmp/ow-question-integrity/target/debug/war and unittest discover.
Evidence: implementation/scheduler-tests.log. Python syntax and diff whitespace
checks also passed.

Browser loop controls, external harness protection production, wider lifecycle
qualification and full release gates remain unfinished. The synthetic receipt
fixture does not prove real deployment isolation or human acceptance. No running
user service was reconfigured or restarted. OW-WAR-0107 remains open.

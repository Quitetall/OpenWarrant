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

# Hotline implementation progress

The first library component defines exact question and answer records. Questions
bind the controller-provided source identity, policy digest and observed committed
checkpoint. Answer authentication derives the responder from a configured token
hash; request bodies cannot nominate their own actor. Governing response scope is
explicit per Warrant. A direct-human request additionally requires a configured
human responder. Delegated AI governing scope does not award a human signature or
qualification. Empty responder configuration grants nobody permission.

Four direct contract tests cover valid answer binding, mutable-input isolation,
stale subject and policy differences, actor injection, credential refusal,
governing scope, direct-human distinction, invalid checkpoints, qualification
injection, duplicate credentials and wildcard scope refusal.

These are library observations only. HTTP storage, responder routing, browser
controls, checkpoint/resume lifecycle, cumulative budgets and integration tests
remain unfinished. No runtime completion or security deployment is claimed.
Bearer credential custody belongs to the configured harness/operator; a token
hash is not proof of human presence or secure acceptance.

Execution now accepts oh.war/execution-question/v1 only after successful harness
exit and a clean committed checkpoint descended from the configured base. It
retains the question in the stopped, blocked attempt. Ordinary start refuses to
bypass that question. A real HTTP/process/Git test observed checkpoint retention,
restart persistence, absent completion signal and admission/start refusal. The
38-test execution suite passes. Answer delivery and explicit resume remain open.

Authenticated answer storage is now connected to HTTP. Tests exercise missing and
wrong responder credentials, actor injection, stale question digest, immutable
answer replay, conflicting answer refusal, restart persistence and session refusal.
All 38 execution-suite tests pass after this change; four contract tests also pass.
Answers alone do not permit another start. Resume and browser controls remain open.

Explicit resume is now wired through the authenticated API. It requires an exact
question digest, retained answer under unchanged responder configuration, unchanged
execution/source policy, stopped prior writer and clean exact checkpoint. The
retained child claim makes resume replay idempotent. The resumed v2 request carries
question/answer history and remaining active-time budget; a continuation does not
spend a repair cycle. HTTP tests run with zero repair cycles, observe reduced time,
refuse revoked responder configuration, changed timeout and dirty checkpoint, then
complete through required checks. All 79 web-package tests pass. Browser controls,
automatic adviser routing and additional time/concurrency faults remain open.

Browser controls now show exact question basis, configured eligible responders,
answer/evidence form and explicit resume. Credential is cleared before submission;
lock removes hotline contents. The retained browser observation demonstrates
credential refusal, answer-without-start, resume, completion and generated report.
No real-provider or secure-human qualification is claimed.

Additional real HTTP/process/Git faults pass in the 42-test execution suite:
concurrent resume requests retain one child attempt; consumed active time reduces
the resumed allowance and timeout remains unknown without a completion signal;
replay cannot replace that unknown writer; a pending answered-but-unresumed
question blocks dependents while an independent Warrant completes; editing the
source after answering refuses resume. No manually fabricated success records
were used to establish these scenarios. Automatic adviser routing remains open.

Configured automatic technical advice is implemented. Exact source/question basis
travels to a bounded process; configured AI identity is assigned by the controller.
Governing and direct-human questions are excluded. Immutable attempt records stop
restart duplicates; unknown advice blocks further automatic calls. Unknown cost
refuses a hard cap before process launch. Answers never resume implementation.
All 89 web tests pass, including live-question routing, initial startup routing,
no governing/direct-human dispatch, timeout persistence and authority-field refusal.
JavaScript syntax passes. Synthetic tests do not qualify real models or containment.
Full gate, exact-head integration and any remaining stage-granularity limits remain
open; no completion or assurance claim is made by this implementation note.

A focused real-process interruption test stops the service while its adviser is
still alive, then restarts. The initial sequence-one record becomes unknown, no
answer appears and the launch marker remains one. The test explicitly terminates
only its own synthetic process group afterward. This proves restart refusal, not
remote-job fencing or a recovery mechanism.

Full isolated gate at 78d2371e05f475270e887659e2812ebf8ebbf0ba passes 14/14
steps and 308 controls on Rust 1.97.1, exit 0. This predates the interruption test
and stage-planner work; it is not proof of later integration.

A workflow-local stage planner now validates explicit bounded DAGs, rejects
cycles/dangling references and inconsistent observations, propagates question
blocks, and distinguishes independent readiness from writer serialization. It
does not parse or replace standard milestone acceptance records. Four library
tests pass. Stage dispatch/configuration/UI integration is not implemented yet;
planner readiness explicitly grants no dispatch permission.

Stage plan configuration now requires explicit title, outcome, dependency graph
and check commands. Shared argv validation moved into the process transport module
without changing existing imports or behavior. Five stage tests and the 95-test
web regression suite pass. STAGES.md records the remaining dispatch, final
aggregation and legitimate checkpoint-advancement requirements. V1 runtime
configuration still refuses unsupported stage plans; no readiness claim is added.

Stage completion projection now requires exact current code revision, source digest,
plan identity and observed matching check commands with integer zero exit results.
A completed label alone, unknown execution, stale code, missing prerequisite
evidence or unrelated checks cannot unblock successors. Six stage library tests
pass. The checkpoint producer and dispatch integration remain unfinished; this
projection consumes performer observations and does not award assurance.

Stage checkpoint production now runs real configured commands on an exact clean
Git revision within the remaining deadline. It preserves check exit results,
rechecks worktree/HEAD after every command, and returns unknown on mutation or
timeout. Eleven stage-library tests pass using real repositories and processes,
including failed prerequisite plus passing successor, dirty/moved revision,
mutation and time exhaustion. Controller dispatch and stage UI remain unwired.

### Stage prerequisite check refusal

Found that checkpoint collection ran successor checks after a prerequisite failed.
Reproduced with a dependent process writing an external fixture marker. Changed
collection to skip dependent checks transitively while running independent checks.
Retained observed failed exit code and explicit skipped prerequisite identities.
Twelve stage tests pass, including actual process non-launch and independent
completion. This does not complete stage dispatch or hotline checkpoint rebinding.

### Configured stage dispatch integration

Execution configuration v2 now binds an explicit stage plan to each configured
Warrant. Authenticated start/admission requests select exact stage. The controller
serializes writers, checks current prerequisite evidence, shares Warrant time
budget and applies repair counts per stage. Harness request v3 supplies selected
stage and plan. Earlier stage checks rerun after later stage changes. Whole-Warrant
completion requires all stages plus final checks; reporting independently refuses
missing or stale stage evidence. V1 remains supported.

Public HTTP/process/Git tests prove out-of-order refusal, partial stage completion,
one worktree, diminishing time allowance, final completion, and rejection of a
later stage that regresses its prerequisite. The web regression run passed 104
tests in 52.214 seconds; an additional reporting refusal test passed in the focused
four-test reporting suite. Full Rust gate has not yet been rerun for this change.
Browser stage selection and safe resume after independent checkpoint advancement
remain open. Existing exact-checkpoint resume refuses such advancement.

### Browser stage selection

Added authenticated read-only stage-selection endpoint and explicit browser stage
selector. Preview does not dispatch; Start rechecks. Completed stages are labeled
from current evidence. Source changes, editor changes and session lock invalidate
selection. Public HTTP tests cover authentication, zero attempts from preview,
and readiness transitions. Actual browser flow proved dependent refusal, partial
completion, final stage completion and unverified work report. See retained
stage-browser-observation.md and fixture source. Checkpoint-advance resume remains
open; no OW-WAR-0106 completion claim.

### Stage hotline integration and checkpoint lineage

A new real-process test exposed a wrong-level lookup of `affected_stages` in the
controller-bound question envelope. Fixed it in both question validation and
frontier derivation. The test now proves a paused API stage blocks API/UI while an
independent docs stage uses the same worktree and advances its checkpoint.

Added exact input revision to dispatch records and an authenticated read-only
checkpoint-review endpoint. It traces changed checkpoints through uniquely retained
independent stage results; it refuses dirty state and unrelated committed changes.
Tests use actual Git commits and HTTP, including unauthorized preview refusal.
This is not answer reconfirmation or permission to resume. Existing changed-head
resume refusal remains until an eligible responder can explicitly reconfirm against
the exact review digest and that immutable response is consumed by resume.

Checkpoint-lineage web regression: 107 tests passed in 56.105 seconds. Full Rust
gate has not been rerun for this batch.

### Authenticated checkpoint reconfirmation

Added immutable per-checkpoint answer records with original answer and review
digests, responder policy identity and a new question basis retaining its original
question digest. Existing answer authentication and direct-human/governing scope
rules also apply to reconfirmation. Resume supplies original and updated contexts
and still requires explicit dispatch, current lineage, stopped writers and limits.

HTTP/process/Git tests cover absent credential, stale review digest, idempotent
reconfirmation, conflicting answer refusal, original answer preservation, restart
and once-only resume, and final remaining-stage completion. Another test proves
further independent work makes the prior reconfirmation stale and requires a new
one. Browser controls are wired; live reconfirmation browser QA and latest full
Rust gate remain outstanding. No Warrant closure or qualification claim.

## Integrated implementation completion

PR 114 merged as 5dd667d38baf6f49f7e951d6e3366b198c561cd1. Exact-head
hosted gate, reference web checks and Bonsai passed for 03a07a91; downloaded Bonsai
evidence names that head and has no findings. Local Rust 1.97.1 gate passed all
14 steps and 308 plants on implementation commit 5e503d08. Retained gate log,
hosted observation and requirement mapping are under completion-audit/ and
scope-audit.md. Prior report remains preserved. Implementation completed,
unverified; no legacy resolution, human signature or qualification was recorded.

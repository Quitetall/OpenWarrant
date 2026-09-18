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

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

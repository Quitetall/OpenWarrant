# Verification candidate

OW-WAR-0107 is in progress. Verification does not award the assurance mark.
Prototype completion remains available without verification.

## Prepare a job

Start the service with its existing execution configuration plus
`--verifier-config PATH --verifier-issuer PATH`. These paths are operator inputs;
HTTP clients cannot select a command, actor, issuer key or configuration path.

The verifier configuration uses `oh.war/verifier-config/v1` with `performer` and
`verifier` objects, each containing `id` and `argv`, plus `cost_mode` (`free` or
`unknown`), `spend_limit_usd` (number or null), and `timeout_seconds` (1–7200).
Distinct actor names are necessary but do not prove independent execution.
Unknown cost cannot satisfy a mandatory spend cap.

Before verifier dispatch, accounting includes all retained execution and verifier
attempts for the Warrant. The execution timeout is the total active-time ceiling;
the verifier timeout also limits each verification. The smaller configured spend
cap applies. Missing time or unknown cost under a hard cap refuses dispatch before
claim consumption. Prepared jobs spend no budget. Historical verifier jobs without
usage observations cannot be treated as zero-cost, zero-time runs. This accounting
is wired to verifier dispatch and explicit verifier-directed repair dispatch.

The issuer file contains `schema: "oh.war/verifier-issuer/v1"`, `public_key` and
`principal`. The public key must be an SSH Ed25519 key for dispatch authentication.
Keep its private key outside agent access. A signature establishes origin and
binding; the harness must also enforce the protections it attests.

Authenticated routes:

- `POST /api/verification` with exactly `attempt_id` and `verification_id`
  (lowercase UUIDs) prepares a job from a completed current execution.
- `GET /api/verification` lists retained jobs.
- `GET /api/verification/<verification_id>` returns the exact request, admission
  basis and retained state.
- `GET /api/verification/<verification_id>/repair` derives an advisory repair plan
  from retained findings and repair history. It does not dispatch. Missing evidence
  blocks; UNKNOWN and nonrepairable findings escalate; configured cycle limits
  override the default of three. The repair action rechecks current scope,
  candidate, writer claims and aggregate budget before execution.
- `POST /api/verification/<verification_id>/repair` accepts an empty JSON object.
  It rechecks current scope, candidate, writer state, evidence and aggregate limits,
  then dispatches repair through the existing execution harness in the same Warrant
  worktree. Repeated requests return the existing repair attempt. The harness receives
  `oh.war/execution-request/v4` with `verification_repair` containing exact verifier
  findings, failed checks, original candidate and observation digest. Completion
  remains unverified and requires a new independent verification of the new commit.
- `POST /api/verification/<verification_id>/rebuttal` accepts a new `verification_id`,
  `argument`, nonempty `evidence` references and existing `finding_ids` (failed check
  IDs use `check-1`, etc.). It prepares a v2 verification request carrying the exact
  prior observation. Fresh protection evidence and explicit start are still needed.
  It does not change the original verdict or consume a repair cycle. One recheck
  per challenged result is retained; unresolved rechecks set `human_review_required`
  and prevent repair through either the original or recheck result until a current
  human repair decision permits repair through the recheck result.
- `GET /api/verification/<verification_id>/dispute` exposes exact question basis,
  eligible human responders and retained decision. With no configured responder,
  state remains `waiting_for_authorized_responder`.
- `POST /api/verification/<verification_id>/dispute` requires `X-OW-Responder` and
  exactly `question_sha256`, `action`, `reason`, `evidence`. Actions are `repair`,
  `verify_again`, `revise_scope`, or `stop`. Identity and Warrant authority come from
  the configured hotline responder. Decisions are immutable and do not change a
  verdict, award qualification, alter scope or launch work. Actual human presence
  depends on protected credential custody; synthetic fixture credentials prove
  protocol handling only.
  A subsequent repair request may use a current `repair` decision. Its digest is
  retained in `verification_repair.decision_sha256`. Changed responder configuration
  invalidates reuse of the decision. Other decision actions block this repair path;
  none overrides budgets, missing evidence or UNKNOWN observations. The original
  FAIL remains unchanged. Broader stop/scope-change orchestration remains unfinished.
- `POST /api/verification/<verification_id>/reverify` accepts exactly a new
  `verification_id` after a current human `verify_again` decision. It prepares a
  v3 verification request containing `human_recheck`: the retained decision and
  exact prior final record. It cannot replace source, checks, policy or candidate.
  One child check may use each decision; exact preparation replay returns that
  child. Dispatch still requires a fresh signed protection receipt and rechecks
  current responder authority. An unresolved result requires a new human decision;
  no prior FAIL is erased and no repair budget is reset.
  Pending disputes also block unrelated verification claims and sibling-result
  repairs for the same Warrant source and candidate. Preparing another claim does
  not bypass a stop, scope-revision or human-review requirement. This check survives
  service restart. Completed child checks advance the dispute chain; historical
  parent findings remain retained. A different source revision is evaluated under
  its own execution policy and current checks.
- `POST /api/verification/<verification_id>/start` accepts exactly
  `payload_base64` and `signature_base64`. These encode the original signed
  `oh.war/harness-protection/v1` receipt and SSH signature. The receipt binds
  `basis_sha256`, the verification UUID as `nonce`, integer `issued_at_unix` and
  `expires_at_unix` (at most five minutes apart), `evidence_ref`, and `protections`.
  Required protection names are `separate-context`, `read-only-candidate`,
  `protected-checks`, and `protected-control-storage`; each must report `pass`.
  The signature namespace is `openwarrant-harness-protection`.

Preparation is idempotent for the same exact work. Reusing an identity for changed
work refuses. Preparation never launches a process. Current writer uncertainty,
missing prerequisites and stale source/policy refuse preparation. Required
protection evidence remains a separate dispatch condition.

## Current boundary

The internal process controller supports signed protection receipts, a separate
Git repository, protected-check observations, exact-result binding and durable
single-use claims. Start authenticates and consumes a claim before scheduling a
background process; poll the job route for results. Replay never launches again.
Job views separate the historical observation from `effective_verdict`. Missing
or corrupt retained protection receipts set `evidence_state` to `unavailable` and
the effective verdict to `unknown`, while preserving the original observation.
Restoring the exact receipt restores the evidence view without rewriting history.
Another running or uncertain verifier blocks new dispatch. The browser offers
preparation from completed attempts, exact-request download, signed receipt upload,
explicit start and result refresh. Running jobs poll every two seconds; locking
the session clears their view and polling. The prepare/upload/start/result/lock path
has been observed in a disposable browser fixture with synthetic processes and
machine keys. This does not qualify deployment protection. Aggregate loop budgets,
explicit repairs, rebuttals and human-directed repair are wired into the API.
Automatic orchestration and browser controls for repair/dispute remain unfinished.
A consumed claim without
a live process observation is UNKNOWN after interruption; it cannot relaunch.
Tests use synthetic processes and disposable machine keys, not independent human
review or deployment sandbox qualification.

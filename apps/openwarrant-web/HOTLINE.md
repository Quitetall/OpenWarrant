# Hotline workflow (implementation in progress)

The reference executor accepts a stopped question result from a configured
harness. It must exit successfully with no managed descendants and leave a clean
committed worktree descended from the configured base. The controller binds the
question to the observed checkpoint, exact source digest and current policy.
Process-group checks do not prove escaped or remote writers are fenced; the
trusted harness still owns containment.

```json
{
  "schema": "oh.war/execution-question/v1",
  "attempt_id": "<from request>",
  "source_sha256": "<from request>",
  "notes": "Progress committed before asking.",
  "next_steps": ["Answer the question, then resume affected work."],
  "question": {
    "kind": "technical",
    "text": "Which existing parser should this feature use?",
    "direct_human": false,
    "affected_stages": ["STAGE-001"]
  }
}
```

Supported kinds are `technical` and `governing`. This result records blocked work,
not completion. It has no completion signal. Ordinary start refuses to bypass a
retained question, including after a response. Explicit resume rechecks the exact
checkpoint and current configuration before launching. Current controller pauses the Warrant; concurrent stages inside that Warrant
are not yet separately scheduled. Independent Warrants retain their own writers.

## Responder configuration

Optional `--hotline-config /private/path/hotline.json` requires execution config.
An empty responder list leaves questions waiting. Each responder has a distinct
credential hash, configured identity and exact scope for governing answers:

```json
{
  "schema": "oh.war/hotline-config/v1",
  "responders": [{
    "id": "project-architect",
    "kind": "human",
    "token_sha256": "<SHA256 of a private high-entropy responder credential>",
    "governing_warrants": ["00000000-0000-0000-0000-000000000001"]
  }]
}
```

Keep raw credentials out of source, context, command arguments and logs. The
controller reads configuration at startup. Its protection and credential custody
are operator/harness responsibilities. This is authenticated workflow advice, not
cryptographic proof of human presence or a signature for the assurance mark.

`kind: ai` supports technical advice. Explicit owner configuration can delegate
governing answers for named Warrants; it cannot satisfy `direct_human: true`.
No wildcard governing scope is accepted. Answer text cannot grant new permissions,
change effective policy or change the approved source.

## API

Routes require the ordinary service-session bearer token and local-origin checks.

- `GET /api/hotline` lists exact questions, question digests and retained answers.
- `POST /api/hotline/<attempt_id>/answer` additionally requires one
  `X-OW-Responder` header containing the responder credential.
- `POST /api/hotline/<attempt_id>/resume` takes only `question_sha256`. It requires
  a retained answer under the unchanged responder configuration. Ordinary session
  permission permits dispatch; it does not grant permission to answer.

Answer body contains only `question_sha256`, `answer` and `evidence` (a bounded
list of references). The controller derives respondent identity from the
credential. It refuses caller-supplied actor/qualification fields, stale source or
policy, wrong question digest and insufficient governing or direct-human scope.

Answers use immutable checksummed files under private execution storage. Identical
replay returns the original record. A different answer cannot replace that record.
History remains readable after restart. Checksums detect damage; they are not an
authentication boundary against a process able to rewrite protected storage.

## Resume boundaries

Resume requires unchanged source, Warrant policy, execution configuration and
responder configuration. It refuses dirty, missing or moved worktree checkpoints,
unanswered questions and any running/unknown writer. The existing registry still
binds one worktree to one Warrant. The first resumed attempt is published before
launch; replay returns that attempt and cannot launch another worker.

A resumed harness receives `oh.war/execution-request/v2`, with `resume_from` and
`hotline_context` containing retained question/answer pairs. It must support this
version explicitly and follow the exact source and constraints. Its result uses
the existing result or question protocol. Completion still requires every check.

Active time consumed by each segment is subtracted from the same execution time
budget; waiting for an answer consumes none. A continuation does not count as a
repair, but actual retries retain the configured repair limit. The store's attempt
limit also bounds repeated questions. Unknown-cost refusal and free-cost assertion
remain unchanged; this does not implement paid metering.

The workbench hotline section shows exact question basis and configured responder
identities. Enter a responder credential, answer and evidence, then record the
answer. The credential clears before submission. Answering leaves work paused;
Resume answered work is a separate action and rechecks current state. Lock clears
the hotline view. Failed refresh clears the question list.

Observed refusal cases include consumed-time exhaustion, concurrent resume replay,
stale source, revoked responder configuration, changed execution limits and dirty
checkpoint. Independent Warrant execution proceeds while a dependent remains blocked.

Full integration and lifecycle qualification remain pending. Do not advertise the partial API as a complete
hotline or mark OW106 complete from its component tests.

## Automatic technical adviser

Add `--adviser-config /private/path/adviser.json` alongside hotline configuration:

```json
{
  "schema": "oh.war/hotline-adviser/v1",
  "argv": ["/absolute/path/to/trusted-adviser-adapter"],
  "respondent": "configured-ai-adviser",
  "sandbox": "read-only-repository",
  "cost_mode": "unknown",
  "spend_limit_usd": 10,
  "timeout_seconds": 60
}
```

`respondent` must match a configured `kind: ai` hotline responder. The example
refuses execution because unknown cost cannot meet a hard cap. Use `free` only
for a harness that makes no paid calls; this is an owner assertion, not metering.
An explicit null cap permits unknown cost. No raw responder credential is sent to
the adviser. Credentials/provider keys remain in the trusted adapter environment.
The harness must enforce read-only repository access and protect controller state;
the configuration assertion does not establish sandbox qualification.

The controller routes newly retained technical questions and eligible unanswered
questions found at startup. Governing and direct-human questions never use this
automatic path, even if the AI has separately delegated governing scope. The
adviser receives `oh.war/hotline-advice-request/v1` with the exact question,
question digest, source document, technical-only scope and limits. It returns:

```json
{
  "schema": "oh.war/hotline-advice/v1",
  "question_sha256": "<exact request digest>",
  "answer": "Use the existing SDK parser.",
  "evidence": ["<relevant source reference>"]
}
```

Extra fields refuse. The controller rechecks source/policy and records the answer
as the configured AI identity; output cannot choose its actor or claim authority.
A valid answer leaves work paused for explicit resume. This identity identifies
the configured adapter, not independent proof of which model ran.

Advice runs serially, with one immutable attempt per question. Initial records
precede launch. Restart does not repeat a retained attempt. Timeout or interrupted
execution stays unknown and blocks further automatic adviser calls; no replacement
or paid retry is inferred. Failed or refused attempts remain visible and can be
answered through an eligible authenticated responder. Other Warrants can continue.
Process stdout/stderr is bounded; raw streams are not stored. Browser question
cards show adviser state, cost status and failure text.

Synthetic process tests cover initial and live routing, authority-field refusal,
unknown-cost refusal before launch, timeout/restart behavior and no automatic work
resume. Actual model quality, deployment isolation and release qualification remain
separate. A single Warrant's internal stages are still not separately scheduled.

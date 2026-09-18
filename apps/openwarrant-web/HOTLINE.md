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
retained question, including after a response. Explicit resume is not implemented
yet. Current controller pauses the Warrant; concurrent stages inside that Warrant
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

Answer body contains only `question_sha256`, `answer` and `evidence` (a bounded
list of references). The controller derives respondent identity from the
credential. It refuses caller-supplied actor/qualification fields, stale source or
policy, wrong question digest and insufficient governing or direct-human scope.

Answers use immutable checksummed files under private execution storage. Identical
replay returns the original record. A different answer cannot replace that record.
History remains readable after restart. Checksums detect damage; they are not an
authentication boundary against a process able to rewrite protected storage.

Remaining work: automatic adviser routing, explicit resume with current responder
eligibility and checkpoint revalidation, cumulative budgets, browser controls and
full lifecycle qualification. Do not advertise the partial API as a complete
hotline or mark OW106 complete from its component tests.

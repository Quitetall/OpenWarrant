# Drafting: from a sentence to a reviewable Warrant (§74)

Two paths, one gauntlet. Either you are the drafter, or a configured process is.

## You are the drafter

```bash
war plan "add a changelog" --json          # oh.war/draft-request/v1: namespace, existing
                                           # warrants + ADRs, profile, assurance, answers
```

Answer with an `oh.war/draft-proposal/v2` JSON file. The struct is your entire
output surface: an unknown field is refused at parse (`authorized_by` will
never be "ignored"):

```json
{
  "api_version": "oh.war/draft-proposal/v2",
  "proposed_identity": {"title": "Keep a CHANGELOG", "profile": "delivery", "assurance": "basic"},
  "operations": [
    {"op": "create_atom", "role": "intent", "ordinal": 10, "path": "10-intent.md", "body": "# Intent\n…"},
    {"op": "create_atom", "role": "basis", "ordinal": 20, "path": "20-basis.md", "body": "…"},
    {"op": "create_atom", "role": "work_order", "ordinal": 40, "path": "40-work-order.md", "body": "…"},
    {"op": "create_atom", "role": "milestones", "ordinal": 45, "path": "45-milestones.yaml", "body": "schema: \"oh.war/milestones/v1\"\n…"},
    {"op": "create_atom", "role": "assurance", "ordinal": 60, "path": "60-assurance.md", "body": "…"},
    {"op": "add_relation", "relation_kind": "roadmap", "relation_ref": "roadmap://OW-PHASE-2/…"}
  ],
  "evidence_claims": [], "durable_choices": [], "unresolved_questions": [],
  "risk_assessment": "…", "adequacy_attacks": ["…"]
}
```

Rules the gauntlet enforces: every evidence claim classed and cited; a durable
choice carries an ADR draft (`propose_adr`); an unanswered blocker question
stops a non-interactive run; an invented `war://` blocks; a `create_atom` path
is a file name, never a route.

```bash
war plan --proposal draft.json --reviewed             # validate only
war plan --proposal draft.json --reviewed --apply     # create the Warrant, record plan/
```

## A configured drafter

`openwarrant.toml`:

```toml
[plan]
drafter_argv = ["claude", "-p", "--output-format", "text"]   # request on stdin, proposal on stdout
drafter_timeout_secs = 300
drafter_name = "claude-drafter"
```

`war plan "…" --draft --reviewed --apply`. The tree is compared before and
after the drafter runs; a drafter that wrote a file is refused and its
proposal discarded. What is recorded under `docs/warrants/<alias>/plan/`:
`request.json`, `proposal.json`, `drafter.json`, `pipeline.json`, and three
journal events: so OW-WAR-0042's exit is a recorded run, not a claim.

## Scope and review

The connected agent can draft the response itself; no paid drafter is required.
Use a scratch proposal path outside `generated/`. Applying a reviewed proposal is
an agent-permitted draft write, not a human-only act. `--reviewed` records actual
proposal inspection; it must not stand for an inspection that did not happen.
Only unresolved material decisions justify another user question. Use current
schema/help; never pass RC.3 footer source to a legacy atom parser as if supported.

## Local and cloud agents

Use the same `war spec` method with either backend. A connected agent can draft
directly; a configured harness can invoke a local model or a cloud model. Model
selection belongs to that harness, not the document parser or SDK.

For RC.3 source, use [SDK authoring](sdk-artifacts.md), then validate the actual
output and show the readable draft. For legacy corpus proposals, use the v2
transport above. Preserve exact governing context and source revisions; keep
credentials outside the packet. Record actual backend/model identity when known.

Honor the effective cost policy before invoking a provider. Unknown paid cost
refuses a mandatory hard cap; an explicitly uncapped policy may permit it and
records cost as unknown. A configured command is not proof of metering or sandboxing.
Keep drafting distinct from implementation. Exit with a validated draft path and
remaining material questions; claim no signature, completion or qualification.

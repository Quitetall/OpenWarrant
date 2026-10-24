# Skill transport fixtures

These deterministic agents exercise the public drafting and execution seams.
They consume the selected war skill, shared workflow and glossary as exact
source documents. They check the input hashes and required method text before
returning a proposal. They do not interpret arbitrary natural language or
measure model judgment. The scripted verifier is a test double, not assurance
for OpenWarrant or its code.

- `grill`: supplied Q-001 reaches the request and exported note. An unanswered
  blocker leaves a `needs_input` result; the fixture never invents an answer.
- `spec`: settled requirements produce one bounded policy with a positive
  case, a permission-refusal case and frozen request/response contracts.
- `tickets`: a contract precedes its dependent caller. Only the first stage
  delivers a file. The missing required caller must leave the result partial,
  even with the scripted verifier's response.

The full-context cases declare a 20,000-token input budget. The original
12,000-token exploratory runs are retained as `over_budget` observations;
`grill-12k-budget` keeps that limit as a permanent budget control. The
`grill-unanswered` case records questions without creating a Warrant. These
cases do not change the repository's 60,000-token review cap.

The task's `fixture/CONTEXT.md` is an explicit snapshot of the source glossary.
The fixture refuses if it differs from the context packet. Refresh these
snapshots explicitly when the glossary changes; do not silently substitute
new context into an old observation.

Run from the repository root, with the checkout's built `war`:

```bash
war eval run --tasks-dir evals/skills/tasks \
  --drafter evals/skills/fixture-agent.py \
  --verifier conformance/fixtures/verifier/establishes-all.sh \
  --out /path/to/skills-result.json
war eval verify /path/to/skills-result.json --baseline evals/skills/baseline.json
```

Use `--keep` to retain scratch programs for inspection. The harness never
signs. Its simulated human review is disclosed in each result. Exact source
hashes, tool identity and the retained request/proposal/output records belong
with a recorded run. Baseline comparison is a regression check; it does not
qualify a skill or replace the owner's review of its first run.

The checked baseline is the byte-identical result of hosted run 40088204346.
Its source, executable and archive hashes are in `baseline-provenance.json`.
Actual runner paths in its notes are historical observations; use the retained
archives for replay. It is a fixture regression baseline, not qualification.

The scheduled/manual `eval` workflow captures these five cases, their tool
identity and scratch archives. `retain-run.py` preserves actual input/output
programs before rejecting unexpected scores. CI artifact upload runs on failure
too. The original generic baseline comparison remains independent.

# Fresh-agent workflow observations

These cases exercise the daily workflow with a fresh model context for each
project. They complement the scripted transport fixtures in the parent folder.
The checked fixture baseline keeps its historical source and meaning.

## Protocol

Use three separate disposable projects and the same built `war` for each.
Initialize each with `war init`, retain its generated instructions, then read
the current war skill and the server's actual initialize instructions and
`tools/list`. Each agent starts with its assigned user goal and those sources.

| Case | User goal | Observation required |
| --- | --- | --- |
| Bounded change | Personalize a greeting while preserving its declared schema. | Only the allowed source changes, the protected acceptance check passes, work becomes done, and its effective level is reported as Warrant. |
| Failing checks | Assess existing work without editing its source or checks. | One required check passes and another fails. Work remains incomplete and a requested blocking human question is recorded. |
| Order handoff | Finish the worker portion of ready work requiring human sign-off. | Required checks pass, state becomes awaiting_signature, and the report identifies the pending human action. |

The bounded change has `src/greeting.py` returning a mapping with schema
`greeting/v1` and a message. Acceptance requires `Hello Ada` and `Hello Bob`
for those names while retaining the schema. Only that source is writable.
The failing-check case declares two protected Python commands exiting 0 and 1.
The Order declares a protected passing Python check and is created with
`--sign`. Keep protected-file hashes before and after each run.

Each project has a surrounding case directory containing `case.json` with
`war`, `root` and `id` values. The client performs a real stdio MCP call and
appends its exact request and response to that directory's `calls.jsonl`.

```bash
python3 evals/skills/agent-workflow/mcp-call.py /path/to/case war_start '{"target":"t-example","actor":"case-worker"}'
```

Use a separate context and at most twelve MCP calls for each agent. Keep the
source repository read-only to the agents. The model chooses its actions and
produces its report. A separate observer checks the actual code, protected
hashes, executed checks, returned tracker state, question records and pending
human actions against this table.

Retain the exact prompts, skill and tool metadata hashes, binary checksum and
source/CI identity, case inputs, calls and reports. Record failed observations
and fixes before rerunning a case in a new project and model context.

## Evidence scope

These are bounded workflow observations under explicit skill invocation. They
report the model, supplied instructions and outcomes of these cases. The
common assurance mark and release qualification follow their own requirements.
The source stdio regression tests independently check automatic delivery,
explicit json:false, missing-source refusal and CLI/MCP record-check parity.

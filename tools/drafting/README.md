# Local reference drafter

OW101 prepares a narrow adapter for simple five-atom delivery proposals. It is
not a compiler, orchestrator, general planner, authority service or cloud adapter.
It emits unreviewed `oh.war/draft-proposal/v2`; it never applies or signs work.
Legacy OW42's frozen v1 scope remains separate.

Configure a trusted local llama.cpp endpoint and exact model yourself. The adapter
accepts only literal loopback HTTP origins, disables environment proxies and
refuses redirects. Loopback alone does not prove the backend is free or trusted:
the operator must ensure it is an actual local model, not a paid forwarding proxy.
Pin executable/model hashes in retained run evidence. No model is started,
downloaded or selected implicitly.

Only `delivery/basic` requests are supported. Explicit requests for other profiles
or assurance levels fail before any backend call; omitted values use those defaults.
The exact task is sent as the final user message. All remaining request fields are
preserved in a separate JSON context message, including constraints and unknowns.
This layout does not establish semantic fidelity or defeat prompt injection.

The process reads one canonical `oh.war/draft-request/v1` on stdin and writes one
proposal on stdout. Failure writes only a diagnostic to stderr and exits nonzero.
For example, configure the existing `[plan].drafter_argv` with absolute paths:

```toml
drafter_argv = ["python3", "/path/to/tools/drafting/local_drafter.py", "--endpoint", "http://127.0.0.1:18022", "--model", "exact-local-model-name"]
```

Then `war plan "add a changelog" --draft` requests a proposal without applying it.
Retain raw output and run the normal proposal pipeline. Human review required by
an applicable Warrant still happens before application; this adapter supplies no
review flag. Never repair raw evidence and claim the model produced the repair.

Backend-specific JSON schema constrains role, ordinal and filename individually.
Milestone YAML is a fixed adapter template linking M1, STAGE-001 and OBL-001;
model output cannot change it. Other bodies still require content review.
The adapter also checks those fields locally because a backend can ignore grammar.
It refuses incomplete responses, unknown keys, malformed JSON and over-limit text.
This restricted template does not emit architecture decisions, ADRs, relations or
interview records. Tasks needing those capabilities need a fuller adapter, not
hidden decisions in work-order prose. Compiler semantic validation remains required
for content, YAML milestones, obligations and source references.

Limits: 64 KiB input, 256 KiB backend response, 1,600 requested output tokens,
240-second socket timeout. An outer harness must enforce a total process deadline;
a socket timeout is not a hard wall-clock budget against a slow-drip backend.
This script has no model tools or filesystem-write operations. That does not prove
containment or a filesystem audit of the separately managed model service.

Run tests: `python3 -m unittest discover -s tools/drafting -v`.
Synthetic HTTP fixtures prove transport/refusal only. These tests do not establish real-model quality or end-to-end application.
The first constrained-model observation returned structural output, but invented
existing-Warrant scope and malformed milestone YAML. Retained evidence prompted
the fixed milestone template. A second real-model run preserved that template but
still drafted the wrong deliverable. Both failures remain retained; the revised
task/context message layout requires a new observation.

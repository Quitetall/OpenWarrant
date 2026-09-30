# Context management: September 2026 comparison

OpenWarrant's proposed model fits several current research and engineering
patterns. This is design alignment, not measured equivalence to those systems.
Read this when designing context-provider integration or its evaluation; the
[RC.3 context contract](../sas/drafts/1.0.0-rc.3/context-views-and-shared-work.md)
remains the normative source. Recommendations below do not silently amend it.

## What current systems demonstrate

| Pattern | Primary source | OpenWarrant connection and remaining work |
| --- | --- | --- |
| Keep a small active context; retrieve relevant detail when needed. | [Anthropic, effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) describes hybrid retrieval, lightweight pointers, compaction, durable notes and focused subagents. | Conditional pointers, exact required packets and optional background match this structure. Measure missed context and retrieval latency, not token reduction alone. |
| Keep memory outside the model window, with version history and explicit discovery. | [Letta MemFS](https://docs.letta.com/concepts/memfs) uses Git-backed memory files, a small always-loaded area and other files read on demand. Its base file workflow does not require a vector index. | Decisions/domain records, scratchpads and context suppression fit. One source model can support several views without separate databases for each view. |
| Compact older context into summaries while retaining exact originals for expansion. | [Lossless Context Management](https://arxiv.org/abs/2605.04050v1) proposes an engine-managed summary DAG and retrievable underlying messages. | Immutable sources and manifests supply useful foundations. A summary hierarchy and expansion protocol are provider choices still needing implementation and tests. The retained originals are lossless; the summaries are not. |
| Treat large context as data that code can inspect and divide. | [Recursive Language Models](https://arxiv.org/abs/2512.24601v3) investigates programmatic inspection, decomposition and recursive calls over external context. | Typed task requests and bounded queries fit this direction. Recursive orchestration belongs to LAMU or the harness; it is not a reason to put a model or SQL engine inside the OpenWarrant SDK. |
| Filter data and discover tools before placing their output into model context. | [Anthropic, code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp) describes on-demand tool loading and programmatic filtering of results. | SDK primitives and scoped contract packets can support this. Runtime execution, sandboxing and retrieval remain provider/harness responsibilities. |
| Resume from durable progress and verify before claiming completion. | [Anthropic, effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) describes incremental work, progress logs, feature lists and fresh-session checks. | Work stops, progress projections and evidence records address the same failure pattern. Fresh-session recovery must be exercised with real agents. |

The papers report results on their own benchmark settings. Those results do not
establish one universal best context architecture or OpenWarrant's performance.

## What the OpenWarrant model adds

The proposed contract distinguishes required rules from optional memory. Required
rules travel verbatim with their exact sources and required dependencies; semantic
ranking cannot drop them. Unknown applicability includes the rule and marks the
uncertainty. A packet digest proves byte identity, not that its author identified
every semantically relevant rule.

Snapshots, role/access checks, code-derived contract provenance, explicit shared
Warrant revisions and delta acknowledgements address consistency. They do not
prove an agent follows its instructions. Current implementation work must show
that providers construct and deliver these packets correctly.

Matt Pocock's writing guidance complements this model: each pointer states when
to load its target; shared instructions have one source; each action has a
checkable completion criterion. Applying those principles reduces avoidable
document load. Skill activation and execution still need behavioral evaluation.

## Small improvements to evaluate

1. **Explain each selection.** Let the context manifest expose source identity,
   selected unit/span, inclusion reason, dependency path and token contribution.
   Keep optional summaries linked to exact originals. This makes missing or
   excessive context diagnosable without loading the whole corpus.
2. **Make expansion explicit.** For summarized optional material, provide a
   stable expansion handle and coverage statement. Recheck access when expanding.
   Required binding text remains in the delivered packet, not behind a handle.
3. **Reuse stable context.** Test stable ordering and reusable prefixes, with
   changed task content kept separate where the backend benefits. Existing
   source/policy/role cache keys and acknowledged delta bases remain necessary.
   Provider cache support and measured savings vary.
4. **Benchmark before adding machinery.** Compare the same real tasks under a
   small file-and-search baseline, ordinary retrieval and the proposed packet
   provider. Hold model, tools, task inputs and spend controls constant; record
   repeated-run variance. Extra orchestration must earn its time and token cost.

## Evaluation cases

Use the existing CTX cases for required conformance. Add comparative workloads
that stress distractor documents, a wrong API version, changed requirements,
missing dependency edges, an interrupted session, revoked access and a shared
two-repository contract. Include an impossible token budget: it must produce a
visible limit or smaller task, not silent truncation.

Measure task success, requirement violations, selected required-source coverage,
stale-context errors, tool calls, input/output tokens, latency, paid cost and human
interventions. Exact coverage of declared dependencies is deterministically
testable; discovering all relevant requirements remains a separate quality test.
Use retained source packets and evidence to reproduce failures. Do not begin paid
evaluations under a hard spend cap without reliable accounting.

The next useful step is implementation and measurement. The current design does
not need more named context buckets to start that work.

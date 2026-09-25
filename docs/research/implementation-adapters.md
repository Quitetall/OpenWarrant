# OpenWarrant adapters: LAMU, Katana, BLUT, and Liminal

Status: source investigation and design proposal, 2026-09-13. This is not an
accepted specification, a runtime qualification, or authorization to change any
governed record. Related product decisions: [friction interview](../design/openwarrant-friction.md).

## Finding

All four projects expose useful integration boundaries. The owner's corrected
Q14 decision is to release OpenWarrant on its own and keep these adapters in mind
for later integration. LAMU, Katana, BLUT, and Liminal are not required dependencies
of the first release. This supersedes the initial BLUT-first selection.

The investigation remains a map for future integrations. LAMU needs explicit
context scoping and provenance; Katana needs Dispatch and receipt binding; BLUT
needs durable question suspension/resumption and safe agent-artifact cache
behavior; Liminal needs a Warrant profile and parity qualification. These gaps
must be closed before claiming their respective integrated workflows work. They
do not all become first-release work merely because the interfaces were studied.

Q18 selects both first-release coding-agent connections: an already-connected
agent using an OpenWarrant-specific skill, or a configured agent command launched
by OpenWarrant. Both require the same approved-work protocol. This does not select
a replacement agent loop or compiler inside OpenWarrant, or change current
governed ownership rules.

## Evidence scope

Inspected local source, specifications, and adapter declarations. No agent runs,
model calls, provider probes, builds, or runtime qualification tests were run for
this investigation. Remote branches were not fetched. Existing dirty work was
preserved. HEAD identifies each base; working-tree observations below may include
uncommitted changes and are not claims about clean, released versions.

| Project | Actual checkout | Base HEAD | Observed working tree |
| --- | --- | --- | --- |
| OpenWarrant | `/mnt/4tb/OpenWarrant` | `f22ef2f7282e8b5c72f2c4323b300f3b5e16102c` | `feat/battery-split`; existing AGENTS and agent-doc changes, plus this discussion's documents |
| LAMU | `/home/brianklam/local-llm` (Rust workspace: `lamu-rs/`) | `38c43d9395c9d6c658885a9e3b51bdc33c571e6e` | `main`; six modified Rust files, including retrieval and OAuth/provider code, plus untracked research material |
| Katana | `/home/brianklam/Desktop/katana` | `9c2db1c075537feb8afa605a84242cab8995ca04` | `main`; untracked `.serena/` |
| BLUT | `/home/brianklam/blut` | `b7936f1175f918f5294ddf8668abde1d962e1bfd` | `main`; modified `scripts/contract_lint.py`, untracked `python/` |
| Liminal | `/home/brianklam/Desktop/liminal` | `9478f4b57f8bace18313953f18b1a267853037e1` | `main`, three commits ahead of local `origin/main`; dirty execution docs and untracked nested checkout |

The old BLUT path `/mnt/4tb/LamQuant/training/engine` no longer exists. LamQuant
now declares registry BLUT dependencies in [modules.toml](/mnt/4tb/LamQuant/modules.toml:61).
Old adapter evidence naming that engine or its cookbook binary needs a fresh
binary/registry pin before reuse.

## Ownership already specified

[SAS §11](../sas/WAR_Software_Architecture_Specification.md) already assigns:

- OpenWarrant: Warrant schemas, canonical IR, validation, authoring, compilation
  orchestration, projections, and adapters.
- Liminal: source semantics, graph, provenance, Workspace Basis, and eventual
  document/context compilation.
- Katana: agent loop, PromptIR, model/tool calls, capability enforcement,
  confinement, and runtime receipts.
- BLUT: typed computational DAG execution, resource admission, cache, progress,
  and lineage.
- Knowledge Fabric: authoritative institutional identity, lifecycle, and
  controlled actions for registered Warrants. The local-service design must
  reconcile this existing rule; it cannot silently introduce a second authority.

LAMU supplies model and retrieval services around those boundaries. Katana's
accepted [LAMU ADR](/home/brianklam/Desktop/katana/docs/decisions/0004-lamu-seam.md:1)
already assigns model management and routing to LAMU, with direct-provider Katana
operation also supported.

## LAMU: model delegation and candidate context

| Existing surface | Useful adapter point | Required bound |
| --- | --- | --- |
| `lamu serve`: `POST /v1/chat/completions`, `GET /v1/models` | Katana's OpenAI-compatible provider uses LAMU logical model names | Tool calls, streaming, identity, usage, limits, and cancellation need an actual integration test |
| HTTP `POST /v1/memory/recall` with `RecallContract`, exclusions, health requirements, and trace | Fetch candidate background context for a Warrant | Bind authenticated owner and approved repository/source set |
| MCP delegation with explicit prompt/system/context and `ephemeral=true` | Bounded analysis, summaries, or specialist advice | Return a proposal with source references; never treat its text as approval or authoritative source |
| Semantic repository search and repository-rooted ripgrep | Locate potentially relevant code | Current semantic search does not apply the supplied repository boundary |

Source: [HTTP model facade](/home/brianklam/local-llm/lamu-rs/lamu-api/src/openai_compat.rs:79),
[routes](/home/brianklam/local-llm/lamu-rs/lamu-api/src/openai_compat.rs:224),
[recall request](/home/brianklam/local-llm/lamu-rs/lamu-api/src/memory_api.rs:71),
[recall handler](/home/brianklam/local-llm/lamu-rs/lamu-api/src/memory_api.rs:305),
[semantic search](/home/brianklam/local-llm/lamu-rs/lamu-memory/src/rag.rs:337).

Memory results provide IDs, text, source labels, ranking, and timestamps, but do
not constitute an immutable document-revision manifest. OpenWarrant must fetch
and hash approved source bytes, attach revision/range references, and distinguish
retrieved explanation from required governing constraints. MCP memory's fixed
`local` owner is insufficient for a shared authority model. The HTTP surface's
KeyStore mode uses the authenticated principal's user identity; its
unauthenticated/static-token mode provides a row partition, not that identity
guarantee. Sources:
[memory hit](/home/brianklam/local-llm/lamu-rs/lamu-memory/src/lifetime_memory.rs:474),
[MCP owner](/home/brianklam/local-llm/lamu-rs/lamu-mcp/src/lifetime_memory.rs:23),
[HTTP owner](/home/brianklam/local-llm/lamu-rs/lamu-api/src/memory_api.rs:71).

Retrieval budgets are incomplete: `retrieve(query, mode, owner)` selects a fixed
mode budget and checks wall time after an awaited recall. It does not cancel an
in-flight slow recall. Delegation can also auto-discover context and truncate it;
Warrant context requires an explicit selection contract. See
[retrieval engine](/home/brianklam/local-llm/lamu-rs/lamu-memory/src/retrieval_engine.rs:129).

`compact_context` can summarize message history and preserve originals, but its
constraint-preservation instructions are a prompt rather than validated source
lineage. It inserts the summary as a system message. A Warrant adapter must keep
retrieved or summarized content's original trust level and retain mandatory
constraints independently. Sources:
[compaction](/home/brianklam/local-llm/lamu-rs/lamu-mcp/src/compact.rs:33),
[context assembly](/home/brianklam/local-llm/lamu-rs/lamu-mcp/src/context.rs:438).
LAMU's source-pinned context contract remains part of a proposed SAS, not an
implemented guarantee: [revision record](/home/brianklam/local-llm/lamu-rs/docs/sas/revisions/0.1.0-draft.1.toml:1).

Proposed flow: approved immutable source set → scoped retrieval/ranking → optional
summary → OpenWarrant validates coverage and omissions → Katana compiles runtime
PromptIR. Required constraints must survive context optimization. LAMU does not
own Warrant authority, and a memory hit cannot amend an approved document.

## Katana: execute an approved Dispatch

Katana exposes a one-shot CLI with JSON results and session resume, a public
`KernelConfig`/`Kernel::run_turn` Rust seam, ACP, and an MCP surface. Its provider
trait consumes PromptIR; `OpenAiCompat` supports endpoint/model/key configuration
and logical model routing. The LAMU facade and Katana provider have corresponding
wire interfaces in source; no live pairing was tested here.

Sources: [provider trait](/home/brianklam/Desktop/katana/crates/katana/src/provider.rs:252),
[LAMU routing](/home/brianklam/Desktop/katana/crates/katana/src/provider.rs:365),
[OpenAI-compatible provider](/home/brianklam/Desktop/katana/crates/katana/src/provider.rs:1561).

OpenWarrant already has a digest-bound
[StageDispatch](../../crates/openwarrant-core/src/execution.rs) and
[KatanaReceipt](../../crates/openwarrant-core/src/seam.rs) type. A bridge must map
approved Dispatch scope, workspace, capability ceilings, resource limits, and
context into a Katana run, then return runtime-owned observations bound to the
same Dispatch and attempt.

The CLI's [`--result` JSON](/home/brianklam/Desktop/katana/crates/katana/src/exec.rs:269)
contains status, session path, answer, tool-call summaries, and usage. It is not
the required Warrant runtime receipt. Katana's log records per-request PromptIR
hashes and provider identity, plus policy/confinement configuration and chained
events; the adapter still needs an explicit receipt export contract, including
Dispatch binding and a defined treatment of multiple model-request digests.
Sources: [request log](/home/brianklam/Desktop/katana/crates/katana/src/kernel.rs:4026),
[runtime configuration](/home/brianklam/Desktop/katana/crates/katana/src/kernel.rs:830).

OpenWarrant currently has no receipt store wired into resolution:
[`runtime_receipts_match_the_basis`](../../crates/openwarrant-cli/src/resolve.rs)
returns false whenever a Katana or BLUT stage exists. Also,
[`war perform`](../../crates/openwarrant-cli/src/perform.rs) accepts only generic
`agent` stages and supplies no sandbox. Relabeling Katana work to avoid runtime
receipt requirements would misstate the contract.

Needed bridge work: native dispatch, durable run identity, independent workspace
setup, receipt export/ingestion, and hotline checkpoint/answer/resume binding.
Session resume alone does not establish the user's affected-stage suspension
semantics. Runtime success must remain separate from independent verification
and the selected human or policy acceptance act.

Existing sandbox and budget features do not cover every Dispatch constraint.
Bubblewrap provides stronger isolation than the Landlock fallback, which limits
writes but does not restrict reads/network. A writable workspace mount does not
itself protect mandatory fixtures within that workspace. USD ceilings require
prices and do not aggregate child inference spend; bounded profiles need explicit
wall time. The adapter must map each promised bound or refuse it, and provision
performer/verifier workspaces with independently protected expectations. Sources:
[confinement](/home/brianklam/Desktop/katana/crates/katana/src/saya.rs:1),
[pricing](/home/brianklam/Desktop/katana/crates/katana/src/config.rs:155),
[child accounting](/home/brianklam/Desktop/katana/crates/katana/src/kernel.rs:3583).

ACP supplies session load/prompt/cancel and permission requests. Automation must
identify itself as automation; these requests do not replace OpenWarrant's
authority checks or hotline question contract. Also, CLI `--json` emits the event
log after the turn, so it is not the live progress stream a tracker needs.
Sources: [ACP client class](/home/brianklam/Desktop/katana/crates/katana/src/acp.rs:100),
[CLI JSON output](/home/brianklam/Desktop/katana/crates/katana/src/main.rs:1993).

## BLUT: typed DAG execution and computational semantics

BLUT has two relevant layers:

1. Runtime `PlanSpec::compile(&Registry)` resolves registered stages into a
   `CompiledPlan`; `ParallelExecutor::execute(plan, ExecCtx)` runs it. Cookbooks
   register typed stages. Plain PlanSpec edges are ordered producer/consumer
   indices with positional tuple semantics.
2. `blut-graph-core::Compiler` compiles typed semantic graphs with named ports,
   effects, resources, capabilities, and kernel selection into `AuthorizedPlan`.
   This is computational semantics. It does not interpret prose requirements or
   confer human authorization.

Sources: [PlanSpec](/home/brianklam/blut/src/framework/plan_spec.rs:121),
[compile](/home/brianklam/blut/src/framework/plan_spec.rs:364),
[stage interface](/home/brianklam/blut/src/framework/stage.rs:792),
[executor](/home/brianklam/blut/src/framework/executor.rs:5087),
[semantic compiler](/home/brianklam/blut/crates/blut-graph-core/src/compile.rs:1231),
[AuthorizedPlan contract](/home/brianklam/blut/crates/blut-graph-core/src/model.rs:662).

OpenWarrant already has [BLUT lowering](../../crates/openwarrant-cli/src/blut.rs):
stage/argument lowering, an explicit port-kind map, and optional invocation of a
real cookbook binary's `plan check`. Its schema pin is historical. Existing
[2026-09-03 evidence](../warrants/OW-WAR-0027/evidence/blut-plan-check-2026-09-03.md)
establishes a previous typecheck, not current compatibility or run receipts.

Useful progress/evidence surfaces exist: `StageEvent`, status streams, lineage
queries, and optional web launch/status/SSE/cancel routes. The web launch handler
returns a spawned PID/log, so OpenWarrant still needs durable job correlation and
idempotent admission. Sources:
[status](/home/brianklam/blut/src/framework/status.rs:208),
[lineage](/home/brianklam/blut/src/lineage_db.rs:1028),
[web handler](/home/brianklam/blut/crates/blut-web/src/lib.rs:614).

Three constraints matter for the proposed agent workflow:

- `Control` has Continue/KillBranch/Spawn, without durable question suspension.
  Existing plan resume reruns failed/killed work using cache. The parallel
  executor cancels the plan on its first error, so a pending human question
  cannot be encoded as failure without violating Q7. Sources:
  [control](/home/brianklam/blut/src/framework/control.rs:36),
  [failure behavior](/home/brianklam/blut/src/framework/executor.rs:5098).
- The current semantic-to-durable bridge rejects non-pure effects and stateful
  stages, among other restrictions. Agent sessions modifying workspaces need
  additional support or a direct runtime cookbook adapter. Source:
  [bridge restrictions](/home/brianklam/blut/crates/blut-semantic/src/lib.rs:116).
- Agent-produced patches and mandatory verification need explicit cache rules.
  Setting `DETERMINISTIC=false` alone does not bind downstream reuse to actual
  changed output bytes. Bind source/contract/fixture/output versions and rerun
  mandatory checks with cache bypass. Sources:
  [determinism](/home/brianklam/blut/src/framework/stage.rs:588),
  [cache bypass](/home/brianklam/blut/src/framework/executor.rs:458).

Possible later integration: BLUT owns a delegated graph and invokes Katana for
agent execution through LAMU. OpenWarrant compiles approved work and coordinates
governed decisions and evidence. Suspension, effects, identity, and cache
contracts must be established before that integration is qualified. The choice
between a direct runtime cookbook adapter and an extended semantic-to-durable
bridge remains open. Exactly one component should schedule each delegated graph.
Reference BLUT lineage rather than reconstructing a competing lineage record.
Q14 does not require this integration for the first release.

## Liminal: source-to-graph compiler

There is real callable Rust code:

`Utf8HolderView::from_bytes` → `liminal_cst::parse` → `liminal_hir::lower` →
`liminal_cir::resolve` → `DebugGraphV1` serialization.

Source ID, content hash, byte ranges, graph subjects, and Workspace Basis make
this a useful frontend boundary. Sources:
[source view](/home/brianklam/Desktop/liminal/crates/liminal-source/src/view.rs:13),
[HIR lowering](/home/brianklam/Desktop/liminal/crates/liminal-hir/src/schema.rs:279),
[CIR resolution](/home/brianklam/Desktop/liminal/crates/liminal-cir/src/lib.rs:22),
[debug serialization](/home/brianklam/Desktop/liminal/crates/liminal-cir/src/lib.rs:117).

Current limits are material:

- CST currently wraps the text in one root/text token; HIR independently lowers
  emitted source. The implementation is explicitly qualification substrate;
  its existence is not completed Phase 1 qualification. Sources:
  [parser](/home/brianklam/Desktop/liminal/crates/liminal-cst/src/parser.rs:138),
  [execution amendment](/home/brianklam/Desktop/liminal/docs/execution/phase0-amendments.md:16),
  [current packet](/home/brianklam/Desktop/liminal/docs/execution/phase1-suite-review.md:14).
- No Warrant-specific profile or adapter was found in the inspected Liminal
  source/spec/integration trees. OpenWarrant has parity data types and a
  [Warrant describing the adapter](../warrants/OW-WAR-0040/atoms/40-work-order.md),
  but no source adapter in its compiler. Manifests, sidecars, obligation semantics,
  stable Warrant IDs, and cross-document resolution still require domain lowering.
- Workspace `lim expand` emits the first sorted Markdown file after preflighting
  all files. Use the library per source for an experiment. Its debug interchange
  is interim; AI context compilation remains Phase 10, with only a lab stub
  implemented. Sources:
  [CLI expansion](/home/brianklam/Desktop/liminal/crates/liminal-cli/src/format.rs:102),
  [AI lab query](/home/brianklam/Desktop/liminal/crates/liminal-daemon/src/queries.rs:313),
  [AI compilation plan](/home/brianklam/Desktop/liminal/spec/v4/liminal_master_architecture_plan_v4.md:2443).
- `FrozenWorld` is a conformance-harness facility, not a production artifact
  archive. Preserve approved raw bytes through a separate immutable artifact
  contract. Source: [replay harness](/home/brianklam/Desktop/liminal/conformance/src/replay.rs:137).

Proposed first use: an optional read-only frontend that preserves exact source
bytes and explicit identities. Compare canonical Warrant IR over the entire
declared corpus before replacement. Generic graph production alone does not
establish Warrant semantics or safe context omission.

## Proposed shared adapter contract and proof

Extend the existing Dispatch/Submission and context-manifest seams instead of
inventing a second task description. Proposed requirements:

| Boundary | Information or behavior required |
| --- | --- |
| Admission | Warrant revision/digest, stage/attempt, exact workspace and fixtures, authority-policy reference, permitted capabilities, resource bounds |
| Context | Approved source hashes/revisions, mandatory constraints, optional retrieval provenance, declared omissions, effective token budget |
| Run | Idempotency key, runtime identity, durable state, progress cursor, cancellation, checkpoint identity |
| Hotline | Question identity, affected dependency scope, recorded answer/adviser identity, separate authority validation, revision binding, resumable checkpoint |
| Result | Immutable artifacts and digests, runtime receipt references, actual usage and confinement, terminal status; independent verification recorded separately |

These are design requirements to settle, not interfaces all four projects already
implement. Keep consumer UI focused on approve/start, progress, questions, and
acceptance. Installation and adapter setup must respect Q13's zero recurring
manual-record requirement.

Candidate proof for later integrations; this is not the first-release scope:

1. One approved Warrant is admitted by BLUT, which launches an isolated Katana
   attempt through LAMU and returns linked runtime receipts bound to the exact
   Dispatch, stage, attempt, and produced artifacts.
2. Replaying launch after interruption does not start duplicate work or lose the
   association between approval, session, and evidence.
3. A human question pauses affected work and dependents; an independent branch
   continues; an authorized answer or revision resumes the correct checkpoint.
4. Retrieved content outside approved sources is refused; stale or over-budget
   context cannot silently omit required constraints.
5. Separate verifier reruns protected checks on the exact result; forged/stale
   receipts and changed fixtures cannot satisfy acceptance.
6. A BLUT subplan returns real status/artifacts/lineage; a pending question is
   distinguishable from failure. Liminal frontend replacement additionally
   requires full declared-corpus parity and refusal tests.

None of these integration scenarios was executed by this source investigation.

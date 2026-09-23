---
schema: oh.war/atom/v1
warrant_uuid: 01a0cd26-1b69-7123-a9fe-da2516447ada
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS §6.3: the Roadmap states sequencing, dependency, priority and phase,
  and is separate from the SAS. Scheduling changes SHALL NOT require
  architecture changes.
- SAS §6.4: every WAR SHOULD reference the Roadmap item or phase that
  motivates it.
- SAS §6.10: a program has exactly one SAS; its phases are the program's
  milestones; an Objective is achieved when its `exit` Warrant resolves
  satisfied.
- SAS §8: OpenWarrant SHALL make it possible to compile repository, SAS,
  Roadmap, ADR, policy and artifact context.
- SAS §98, the eleven phases with their Exit criteria, and §105,
  `roadmap://<item-id>`.
- OW-ADR-0023 (this Warrant's decision, ordinal 30), OW-ADR-0022 (currency
  by relation; the master document), OW-ADR-0021 (ownership),
  OW-ADR-0016 (re-pin by amendment).
- Prior plans read in full for the mapping below:
  `docs/roadmap/PRODUCTION_ROADMAP.md`, `docs/roadmap/view.json`,
  `docs/sas/drafts/1.0.0-rc.3/roadmap.json` and `phase-plan.md`,
  `docs/design/rc2-implementation-roadmap.json`,
  `docs/design/remaining-work.md`, `docs/design/openwarrant-product-spec.md`
  ("Engineering contracts still to specify", "Friction targets and proof").

## The two candidate phase sets

**A — the eleven §98 phases.** SAS 1.1.0 accepts them, every one carries an
Exit, and 70 of 110 Warrants already cite one. The four-phase grouping
survives as each phase's `priority`, so release sequencing is not lost.

| id | title |
|---|---|
| OW-PHASE-0 | Telemetry shim |
| OW-PHASE-1 | File-native WAR compiler |
| OW-PHASE-2 | Agent planner |
| OW-PHASE-3 | ADR federation |
| OW-PHASE-4 | Knowledge Fabric registration |
| OW-PHASE-5 | Dispatch and Katana execution |
| OW-PHASE-6 | Gate Registry and assurance case |
| OW-PHASE-7 | BLUT adapter |
| OW-PHASE-8 | Liminal production compiler |
| OW-PHASE-9 | High-assurance controls |
| OW-PHASE-10 | Contractor Work Order profile |

**B — the four-phase plan** (rc.3 `roadmap.json`, "proposal-not-authorized").
It is shorter and closer to how releases are actually cut. Every Warrant must
be re-referenced, and the §98 Exit criteria are rewritten as four.

| id | title |
|---|---|
| OW-PHASE-1 | Library and Standard |
| OW-PHASE-2 | SDK and Compiler Integration |
| OW-PHASE-3 | Workflow Integration |
| OW-PHASE-4 | Hardening and Adoption |

**Recommendation: A**, with B's grouping as `priority` (1 = Library and
Standard … 4 = Hardening and Adoption). It moves no signed ref, keeps the
eleven accepted Exits, and loses nothing B orders. The owner decides by
answering Q-001; the rejected set stays here as the record of the choice.

## Every Warrant under both sets

A cell marked *(proposed)* is a Warrant that names no phase today. Signed
Warrants keep their manifests; their placement lives in the roadmap's
mapping and is applied to the ref at their next amendment. OW-WAR-0073 is
superseded and appears as lineage only.

| Warrant | state | A: eleven §98 phases | B: four phases | title |
|---|---|---|---|---|
| OW-WAR-0001 | authorized | PHASE-1/bootstrap | 1 Library and Standard | Establish the OpenWarrant repository and Rust workspace |
| OW-WAR-0002 | resolved | PHASE-1/compiler | 1 Library and Standard | Implement the file-native manifest and atom parser |
| OW-WAR-0003 | authorized | PHASE-1/compiler | 1 Library and Standard | Implement the canonical WAR IR and RFC 8785 digesting |
| OW-WAR-0004 | authorized | PHASE-1/compiler | 1 Library and Standard | Implement the generated parent document and drift checking |
| OW-WAR-0005 | resolved | PHASE-1/bootstrap-closure | 1 Library and Standard | Implement deterministic war check and close the Phase 1 bootstrap |
| OW-WAR-0006 | authorized | PHASE-3/adr-federation | 1 Library and Standard | Complete ADR federation: relations, supersession, and currency |
| OW-WAR-0007 | authorized | PHASE-1/milestones | 1 Library and Standard | Parse and validate milestones, stages, and named typed ports |
| OW-WAR-0008 | resolved | PHASE-1/state-model | 1 Library and Standard | Implement the state model: phase, condition, outcome, currency, standi |
| OW-WAR-0009 | resolved | PHASE-1/contract | 1 Library and Standard | Implement contract revisions and their immutability |
| OW-WAR-0010 | resolved | PHASE-1/autonomy | 1 Library and Standard | Implement the autonomy envelope and amendment records |
| OW-WAR-0011 | authorized | PHASE-5/preflight | 3 Workflow Integration | Implement prerequisites and Preflight |
| OW-WAR-0012 | resolved | PHASE-1/context | 1 Library and Standard | Implement the context model, context manifest, and trust classes |
| OW-WAR-0013 | authorized | PHASE-1/traceability | 1 Library and Standard | Validate SAS and Roadmap traceability |
| OW-WAR-0014 | resolved | PHASE-1/rationale | 1 Library and Standard | Implement the rationale model, assumptions, and unknowns |
| OW-WAR-0015 | resolved | PHASE-5/artifacts | 1 Library and Standard | Implement deliverables, artifacts, and artifact provenance |
| OW-WAR-0016 | resolved | PHASE-6/obligations | 1 Library and Standard | Implement acceptance obligations and bounded claims |
| OW-WAR-0017 | resolved | PHASE-6/epistemic | 1 Library and Standard | Implement the epistemic classes: evidence, observation, inference, jud |
| OW-WAR-0018 | resolved | PHASE-6/adequacy | 1 Library and Standard | Implement contract-adequacy review, structurally checked |
| OW-WAR-0019 | resolved | PHASE-6/gate-registry | 1 Library and Standard | Implement the Gate Registry: definitions, qualification, and bindings |
| OW-WAR-0020 | resolved | PHASE-6/gate-runs | 1 Library and Standard | Implement Gate Run semantics, askability, and invalidation |
| OW-WAR-0021 | resolved | PHASE-6/independence | 1 Library and Standard | Implement verifier independence |
| OW-WAR-0022 | resolved | PHASE-6/resolution | 1 Library and Standard | Implement resolution, dispute, and annulment |
| OW-WAR-0023 | authorized | PHASE-5/dispatch | 2 SDK and Compiler Integration | Implement Stage Dispatch compilation and actor-specific projection |
| OW-WAR-0024 | resolved | PHASE-5/attempts | 3 Workflow Integration | Implement Stage Submission and attempt semantics |
| OW-WAR-0025 | resolved | PHASE-5/blockers | 3 Workflow Integration | Implement blockers, deviations, decision proposals, and discovered gap |
| OW-WAR-0026 | authorized | PHASE-5/katana | 3 Workflow Integration | Implement the Katana runtime seam, capabilities, and receipts |
| OW-WAR-0027 | authorized | PHASE-7/blut | 3 Workflow Integration | Implement the BLUT adapter: PlanSpec lowering and lineage receipt |
| OW-WAR-0028 | authorized | PHASE-4/kf-actions | 2 SDK and Compiler Integration | Implement Knowledge Fabric typed actions and the controlled-action env |
| OW-WAR-0029 | authorized | PHASE-4/federation | 2 SDK and Compiler Integration | Implement KF registration, global identity allocation, and federation |
| OW-WAR-0030 | authorized | PHASE-4/preservation | 2 SDK and Compiler Integration | Implement portable preservation: one-file export and round trip |
| OW-WAR-0031 | resolved | PHASE-1/journal | 1 Library and Standard | Implement the local draft journal |
| OW-WAR-0032 | authorized | PHASE-4/schema-pack | 2 SDK and Compiler Integration | Generate the schema pack and implement protocol versioning |
| OW-WAR-0033 | resolved | PHASE-1/projections | 1 Library and Standard | Implement the remaining read projections |
| OW-WAR-0034 | resolved | PHASE-2/agent-protocol | 3 Workflow Integration | Implement the agent protocol and Draft Proposal validation |
| OW-WAR-0035 | authorized | PHASE-2/plan | 3 Workflow Integration | Implement `war plan` and the interview loop |
| OW-WAR-0036 | authorized | PHASE-3/decision-detection | 2 SDK and Compiler Integration | Implement normative-decision detection and proposed-ADR generation |
| OW-WAR-0037 | authorized | PHASE-2/diff | 3 Workflow Integration | Implement `war diff`: semantic difference between revisions |
| OW-WAR-0038 | resolved | PHASE-3/importer | 1 Library and Standard | Implement the existing-ADR importer, preserving unknown classes |
| OW-WAR-0039 | authorized | PHASE-0/telemetry | 3 Workflow Integration | Implement telemetry, unit economics, and untracked-work detection |
| OW-WAR-0040 | authorized | PHASE-8/liminal | 2 SDK and Compiler Integration | Implement the Liminal adapter and measured parity harness |
| OW-WAR-0041 | authorized | PHASE-0/exit | 3 Workflow Integration | Discharge the Phase 0 exit: real telemetry distributions, with a basel |
| OW-WAR-0042 | authorized | PHASE-2/exit | 3 Workflow Integration | Discharge the Phase 2 exit: a vague request becomes a reviewable draft |
| OW-WAR-0043 | authorized | PHASE-3/exit | 4 Hardening and Adoption | Discharge the Phase 3 exit: migrate the LamQuant ADR corpus, fabricati |
| OW-WAR-0044 | authorized | PHASE-4/exit | 2 SDK and Compiler Integration | Discharge the Phase 4 exit: KF is institutional authority, Git stays S |
| OW-WAR-0045 | authorized | PHASE-5/exit | 3 Workflow Integration | Discharge the Phase 5 exit: a stateless actor executes one Dispatch |
| OW-WAR-0046 | resolved | PHASE-6/exit | 1 Library and Standard | Discharge the Phase 6 exit: a delivery closes only through bounded pro |
| OW-WAR-0047 | authorized | PHASE-7/exit | 3 Workflow Integration | Discharge the Phase 7 exit: compatible WARs execute without duplicatin |
| OW-WAR-0048 | authorized | PHASE-8/exit | 4 Hardening and Adoption | Discharge the Phase 8 exit: measured adapter parity and the two-host c |
| OW-WAR-0049 | authorized | PHASE-1/residue | 1 Library and Standard | Close the alpha residue: the gaps alpha carried forward and one false  |
| OW-WAR-0050 | authorized | PHASE-6/evidence *(proposed)* | 3 Workflow Integration | Governed Bonsai evidence for pull-request workflow |
| OW-WAR-0055 | resolved | PHASE-1/projections | 1 Library and Standard | Compute the goal hierarchy: war status and the corpus projection |
| OW-WAR-0056 | resolved | PHASE-5/dispatch | 3 Workflow Integration | Compile a real Stage Dispatch (§47) |
| OW-WAR-0057 | resolved | PHASE-1/projections | 1 Library and Standard | The viewer: a static page over the corpus projection |
| OW-WAR-0058 | resolved | PHASE-3/sas-governance | 1 Library and Standard | Put the SAS under §101 governance: controlled revisions and the Releas |
| OW-WAR-0059 | authorized | PHASE-6/gate-runs | 1 Library and Standard | Commit gate receipts as evidence and record the first resolutions |
| OW-WAR-0060 | resolved | PHASE-1/projections | 1 Library and Standard | Publish the corpus viewer: GitHub Pages from the committed projection |
| OW-WAR-0061 | resolved | PHASE-1/exit | 1 Library and Standard | Discharge the Phase 1 exit: OpenWarrant development uses WARs |
| OW-WAR-0062 | resolved | PHASE-3/definitions | 1 Library and Standard | Lock down the levels: what a SAS, a Warrant, and every other object ar |
| OW-WAR-0063 | authorized | PHASE-6/conformance | 1 Library and Standard | Complete the conformance battery: the plants and tests the verifier na |
| OW-WAR-0064 | authorized | PHASE-3/corrections | 1 Library and Standard | Repair a delivered artifact: the correction act for a resolved Warrant |
| OW-WAR-0065 | authorized | PHASE-5/context | 2 SDK and Compiler Integration | Research memo: which tokenizer approximation OpenWarrant uses for §33. |
| OW-WAR-0066 | authorized | PHASE-6/conformance *(proposed)* | 3 Workflow Integration | Run: the conformance battery as a service stage |
| OW-WAR-0067 | authorized | PHASE-9/release | 4 Hardening and Adoption | Release 1.0: the seven acts that close the plan, deferred to the owner |
| OW-WAR-0068 | authorized | PHASE-2/plan | 3 Workflow Integration | Skills: a strict superset of the Pocock set over the deterministic cor |
| OW-WAR-0069 | authorized | PHASE-5/dispatch | 3 Workflow Integration | The board, the hotline, and the harness: one system from master record |
| OW-WAR-0070 | authorized | PHASE-2/plan | 3 Workflow Integration | Add `war inbox` to list Warrants awaiting a human act |
| OW-WAR-0071 | authorized | PHASE-9/release | 4 Hardening and Adoption | SAS 1.1.0: the batch act, and the dashboard as a rendering of the reco |
| OW-WAR-0072 | authorized | PHASE-9/release | 4 Hardening and Adoption | The batch act: one human signature over many pending acts |
| OW-WAR-0073 | superseded | → OW-WAR-0112 | → OW-WAR-0112 | war tui: the dashboard in the terminal |
| OW-WAR-0074 | authorized | PHASE-3/sas-governance *(proposed)* | 1 Library and Standard | Adopt the exact RC.2 build basis without rewriting history |
| OW-WAR-0075 | authorized | PHASE-1/documents *(proposed)* | 1 Library and Standard | Parse and validate human-first OpenWarrant documents |
| OW-WAR-0076 | authorized | PHASE-1/documents *(proposed)* | 1 Library and Standard | Author and edit human-first OpenWarrant documents |
| OW-WAR-0077 | authorized | PHASE-5/context *(proposed)* | 2 SDK and Compiler Integration | Define SDK source types and prove provider resolution integration |
| OW-WAR-0078 | authorized | PHASE-5/context *(proposed)* | 2 SDK and Compiler Integration | Validate condition syntax and prove provider evaluation integration |
| OW-WAR-0079 | authorized | PHASE-5/context *(proposed)* | 2 SDK and Compiler Integration | Prove provider context selection and dependency closure |
| OW-WAR-0080 | authorized | PHASE-5/context *(proposed)* | 2 SDK and Compiler Integration | Prove provider master context and task projections |
| OW-WAR-0081 | authorized | PHASE-5/context *(proposed)* | 2 SDK and Compiler Integration | Inspect packet integrity and prove offline provider packages |
| OW-WAR-0082 | authorized | PHASE-5/context *(proposed)* | 2 SDK and Compiler Integration | Prove provider context budgets and cache invalidation |
| OW-WAR-0083 | authorized | PHASE-6/records *(proposed)* | 1 Library and Standard | Evaluate supplied work and assurance records |
| OW-WAR-0084 | authorized | PHASE-3/lineage *(proposed)* | 1 Library and Standard | Preserve legacy history and explicit successor lineage |
| OW-WAR-0085 | authorized | PHASE-1/sdk *(proposed)* | 1 Library and Standard | Prove Phase 1 SDK, CLI and adapted skill scope |
| OW-WAR-0086 | authorized | PHASE-5/lamu *(proposed)* | 2 SDK and Compiler Integration | Integrate SDK with LAMU and a second minimal consumer |
| OW-WAR-0087 | authorized | PHASE-1/cli *(proposed)* | 1 Library and Standard | Expose document and record primitives through CLI |
| OW-WAR-0088 | authorized | PHASE-2/authoring *(proposed)* | 2 SDK and Compiler Integration | Make SDK document authoring interactive and recoverable |
| OW-WAR-0089 | authorized | PHASE-9/release *(proposed)* | 4 Hardening and Adoption | Prepare standard and SDK release artifacts |
| OW-WAR-0090 | authorized | PHASE-9/qualification *(proposed)* | 4 Hardening and Adoption | Qualify four-phase release and freeze Stable source set |
| OW-WAR-0091 | authorized | PHASE-9/stable *(proposed)* | 4 Hardening and Adoption | Publish and verify OpenWarrant Stable 1.0 |
| OW-WAR-0092 | authorized | PHASE-1/projections *(proposed)* | 1 Library and Standard | View project progress as offline HTML and a locally refreshed dashboar |
| OW-WAR-0093 | authorized | PHASE-5/workflow *(proposed)* | 3 Workflow Integration | Reference workflow package: local API and draft webapp |
| OW-WAR-0094 | authorized | PHASE-5/workflow *(proposed)* | 3 Workflow Integration | Connected-agent execution workflow and model-neutral drafting roadmap |
| OW-WAR-0095 | authorized | PHASE-5/workflow-study *(proposed)* | 3 Workflow Integration | Three-developer reference workflow qualification study |
| OW-WAR-0096 | authorized | PHASE-9/authority *(proposed)* | 4 Hardening and Adoption | Authenticate authority changes through signed transitions and protecte |
| OW-WAR-0097 | authorized | PHASE-1/console *(proposed)* | 1 Library and Standard | Read-only repository doctor and actionable readiness diagnostics |
| OW-WAR-0098 | authorized | PHASE-5/workflow *(proposed)* | 3 Workflow Integration | Share reference workflow start checks with read-only admission preview |
| OW-WAR-0099 | authorized | PHASE-5/frontier *(proposed)* | 3 Workflow Integration | Preserve frontier integrity when stage history is damaged |
| OW-WAR-0100 | authorized | PHASE-0/telemetry *(proposed)* | 3 Workflow Integration | Report unsupported telemetry as unknown without changing historical ba |
| OW-WAR-0101 | authorized | PHASE-2/plan *(proposed)* | 3 Workflow Integration | Constrain local drafting responses before semantic validation |
| OW-WAR-0102 | authorized | PHASE-5/work-stop *(proposed)* | 3 Workflow Integration | Generate deterministic execution work-stop reports |
| OW-WAR-0103 | authorized | PHASE-5/hotline *(proposed)* | 3 Workflow Integration | Preserve question-store errors across hotline readers |
| OW-WAR-0104 | authorized | PHASE-5/hotline *(proposed)* | 3 Workflow Integration | Complete read-only master board and batch question workflow |
| OW-WAR-0105 | authorized | PHASE-2/plan *(proposed)* | 3 Workflow Integration | Draft human-first Warrants through configured agent harnesses |
| OW-WAR-0106 | authorized | PHASE-5/hotline *(proposed)* | 3 Workflow Integration | Route workflow questions and resume bounded work after answers |
| OW-WAR-0107 | authorized | PHASE-6/verification *(proposed)* | 3 Workflow Integration | Independently verify workflow results and repair within limits |
| OW-WAR-0108 | authorized | PHASE-7/blut *(proposed)* | 3 Workflow Integration | Integrate shared BLUT materialization repair and retained OW47 runtime |
| OW-WAR-0109 | authorized | PHASE-5/workflow *(proposed)* | 3 Workflow Integration | Durable agent availability queue and automatic bounded dispatch |
| OW-WAR-0110 | authorized | PHASE-4/schema-pack *(proposed)* | 2 SDK and Compiler Integration | Generate OpenWarrant TypeScript record types and integrate KF consumpt |
| OW-WAR-0111 | authorized | PHASE-4/preservation *(proposed)* | 2 SDK and Compiler Integration | Implement content-bearing preservation archives and honest import roun |
| OW-WAR-0112 | authorized | PHASE-1/console | 1 Library and Standard | war is the app: ownership, a guided start, and remedies |
| OW-WAR-0113 | draft | PHASE-1/projections | 1 Library and Standard | Two projections from atoms by relation: the current master document, a |

## The gaps, placed

Each becomes a phase slug marked "no Warrant yet" in roadmap revision 1.
The slug is the name the Warrant written for it will cite.

| gap | source | A | B |
|---|---|---|---|
| RQ-001 immutable UUIDv7 identity; RQ-002 aliases never substitute | §106, unaddressed | PHASE-1/identity | 1 |
| RQ-023 child cites the exact parent revision | §106, unaddressed | PHASE-3/parent-revision | 1 |
| RQ-046 a Dispatch over budget is refused | §106, unaddressed | PHASE-5/budget-refusal | 2 |
| RQ-060 KF owns authority and lifecycle | §106, unaddressed | PHASE-4/kf-authority | 2 |
| RQ-065 native systems retain artifact authority | §106, unaddressed | PHASE-4/native-authority | 2 |
| Phase 10 has no Warrant; its Exit is unowned | §98 | PHASE-10/contractor-profile, PHASE-10/exit | 4 |
| Phase 9 has no exit Warrant | §98 | PHASE-9/exit | 4 |
| An independent verifier pipeline (every Warrant warns `independence.insufficient`) | `war check`, 2026-09-23 | PHASE-6/verification-pipeline | 3 |
| Batch signing built (OW-WAR-0072 authorized, unbuilt) | OW-WAR-0072 | PHASE-9/release (exists) | 4 |
| Retention: compacting journals, archiving resolved Warrants, a budget at 1,000 Warrants | owner's question, 2026-09-22 | PHASE-1/retention | 4 |
| Teams: review assignment, a multi-person queue | product spec | PHASE-9/teams | 4 |
| Measuring the friction targets (setup ≤10 min, routine ≤60 s) | product spec | PHASE-0/friction-baseline | 3 |
| Brownfield adoption: a guided path for a repository with no Warrants | 2026-09-23 | PHASE-3/adoption | 4 |
| SAS atomization | OW-WAR-0113 non-goal | PHASE-3/sas-atoms | 1 |
| Storage, crash recovery, retained-artifact migration | product spec contracts | PHASE-1/storage | 4 |
| Idempotency, batch atomicity, version negotiation | product spec contracts | PHASE-5/idempotency | 2 |
| Human and session authentication, protected policy state | product spec contracts | PHASE-9/authentication | 4 |
| Agent and harness adapters per OS, writer handoff, cancellation | product spec contracts | PHASE-5/adapters | 3 |
| Compiler source, conflict and omission rules; evidence reuse after source change | product spec contracts | PHASE-6/evidence-reuse | 2 |
| Time and spend defaults, repair accounting, hotline delivery with no responder | product spec contracts | PHASE-5/hotline-defaults | 3 |
| Acceptance validity when an accepted candidate changes before merge | product spec contracts | PHASE-6/acceptance-validity | 3 |
| Assurance baseline, mark issuance and binding | product spec contracts | PHASE-6/assurance-mark | 4 |
| Document schema and Markdown/header grammar; standard vs tool conformance | product spec contracts | PHASE-1/document-grammar | 1 |
| Phase 2/3 exit criteria for the SDK track; KF Compiler interface | product spec contracts | PHASE-2/exit (exists), PHASE-4/kf-compiler | 2 |

## Assumptions

- A-001: the §98 text stays in the SAS until the SAS revision that points
  it at the record is accepted. Until then `status.rs` reads the record when
  one exists and §98 otherwise. Confidence: high.
- A-002: placing a signed Warrant in the roadmap's mapping without editing
  its manifest satisfies §6.4's SHOULD for that Warrant. The relation still
  lives on the Warrant once it is next amended. Confidence: medium; the
  alternative, 70 amendments, is the friction this program exists to
  remove.

## Constraints

- No Warrant's signed contract moves. `war roadmap assign` refuses a
  signed Warrant and names the amendment path.
- The roadmap record adds a schema (`oh.war/roadmap/v1`) and the pack
  version moves once, in M2, with a regenerated pack.
- Pinned files declared in `deliverables.toml` are edited only after this
  Warrant is authorized (OW-ADR-0021).

## Residual risks

- R-001: an owner who answers Q-001 with B re-references 70 Warrants. The
  mapping above is the whole of that work, and M2's `assign` applies it to
  the unsigned ones.
- R-002: a roadmap revision that removes a phase with members.
  `roadmap.unknown-phase` fires on every member, by name. That is the
  intended friction.

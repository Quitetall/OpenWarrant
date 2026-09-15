# OpenWarrant: document standard and low-friction toolchain

> **2026-09-14 update:** The owner directed completion of SAS **1.0.0-rc.2**
> and testable Phase 1 scope before building. The [RC.2 source set](../sas/drafts/1.0.0-rc.2/README.md)
> now consolidates these decisions, chooses the candidate format and assurance
> contracts, and provides examples and a build matrix. This document retains the
> earlier interview/research history; statements below that a mechanism remained
> open describe that earlier stage. Use RC.2 for the current candidate design.
> RC.2 is not yet accepted or implemented.

Status: proposed product requirements, 2026-09-13. The owner reopened product
fundamentals before SAS revision planning; Q36-Q50 and owner clarifications
distinguish document validity, tool workflows, and an optional assurance mark.
The owner's C11 directive defines three development phases, with Knowledge
Fabric Compiler as the first Workflow-phase integration. C12 places the official
standard-and-compiler v1.0 milestone at completion of the CLI/Compiler phase.
C13 directs integration of the reviewed context improvements into the SAS. C14
keeps the working SAS at 1.0.0 and designates the earlier baseline 1.0.0-rc.1.
The [working SAS](../sas/drafts/1.0.0-rc.2/WAR_Software_Architecture_Specification.md)
now contains the context and packet requirements; wider consolidation continues.
Mutual product grilling continues; shared understanding is not yet confirmed.
This synthesizes the owner's [interview decisions through Q53](openwarrant-friction.md)
and earlier product direction. It is input to a revision of the existing program
SAS; current governed authority changes through its established adoption process.

## Purpose

OpenWarrant's core owned surface is the parsable document standard for agentic
work. Generators, compilers, and workflows consume that standard; their builders
control how those tools operate. OpenWarrant can provide supporting tools without
owning every implementation that uses its documents.
The owner considers the first official standard format still under development,
not a finalized stable 1.0 contract. This product direction does not change any
existing accepted SAS revision or governed record. (Q36)

The tools turn a reviewed implementation plan into work agents can carry out,
question, independently verify, and return for acceptance. The primary goal is
to reduce developer coordination while making AI-assisted changes safer in real
workflows. Different authoring interfaces must serve the same standard.

The previously selected reviewed workflow is:

**Prepare → approve and start → watch progress / answer questions → verify → accept.**

The owner's later assurance clarification also permits autonomous prototyping
under human-granted configuration and permissions, without imposing this entire
reviewed workflow on every experiment. Earning the assurance mark is a separate
claim with stronger requirements.

The system assembles context, manages records, and routes decisions. Developers
spend their attention on intended behavior and results.

## Standard and tool ownership

The owner clarified that OpenWarrant owns the barebones document standard, with
only partial involvement in generators, compilers, and workflows built around
it. Using the standard does not transfer control of those tools to OpenWarrant.
A compatible document follows shared structure and defined field meanings,
including what scope and constraints describe and which document revision an
approval refers to. Tools choose their own drafting, compilation, and execution
workflows. OpenWarrant owns the document contract; it does not prescribe every
tool's operating procedure. (Q40)

A small, honestly labeled draft can be a valid OpenWarrant document without
fixtures, approval, or execution records. Document validity and readiness to
execute are distinct: the chosen workflow requires additional records before
acting. A valid draft makes no claim of authorization, verification, or completed
work. Title, outcome, scope, and context illustrate the lightweight shape; the
exact minimum field set still needs definition. (Q41)

This draft retains earlier workflow decisions as proposed reference-tool
behavior. They do not automatically become conformance requirements for every
tool consuming an OpenWarrant document. The precise required fields and assignment
of individual requirements to the core, optional profiles, or tool/repository
policy remain open. C11 stages supporting tools after directly testable features
and libraries. Q50's TUI-before-browser preference does not make either client
a prerequisite for the earlier phases. C12 places the official standard and
compiler v1.0 at the end of Phase 2; Workflow-phase integrations follow.
Exact conformance and phase-exit criteria remain to be specified.
(Q36, Q50, C11-C12)

## Optional assurance and autonomous work

The document standard may be used for fully automated work, including workflows
that do not meet serious-development standards. An agent may carry out Warrants
within permissions and configuration granted by an authorized human; this need
not require a fresh human review for each Warrant. Permission to run does not
itself constitute human acceptance of the result. (Owner clarification C10)

A separate, as-yet-unnamed tag or mark distinguishes work that meets a defined
set of development standards and receives actual human review and acceptance.
"OpenWarrant Verified" was an illustrative name, not a selected term. The owner
requires human acceptance for a Warrant to be signed as verified or complete.
An agent finishing implementation or passing checks alone does not establish
that signed status or earn the mark. Format validity alone does not earn it.

Prototyping should remain low-friction until the team seeks acceptance and
integration into main or production. A prototype can qualify later under
standards that assess the final result: add acceptance fixtures, independently
verify the result, and obtain human review and acceptance, preserving the actual
development history. A standard requiring fixtures before implementation remains
unmet when they were added afterwards. Later qualification does not retroactively
establish an earlier development practice. (Q43)

The mark covers one accepted Warrant result, tied to its exact code revision and
stated scope. A repository may display these records; one marked change does
not imply that the entire codebase satisfies the standards. Whole-release or
repository-wide qualification is not the selected unit for this mark. (Q44)

OpenWarrant defines a versioned baseline for the common assurance mark.
Repositories may strengthen its requirements but cannot weaken them while
retaining that same mark. Teams remain free to use other workflows without the
mark; the optional assurance baseline is distinct from document validity. (Q45)

For the baseline mark, a human reviews the outcome, verification findings, and
remaining risks, inspects code where needed, and explicitly accepts the exact
result. Independent verification supplies detailed technical review. Repositories
may additionally require human review of the full code diff. The baseline human
act does not claim that every line received human inspection. (Q46)

Finished prototype Warrants may remain "implementation finished, unreviewed"
without generating mandatory human acceptance tasks. Request review when the
user seeks qualification or repository policy requires it. That status carries
no assurance mark and is not signed Warrant completion. (Q47)

The remaining baseline requirements and evidence required for each requirement
remain open. Earlier automatic-acceptance choices cannot be interpreted as
permission to award the human-backed mark without the required human act.

The workflow below records earlier reference-tool choices. C10 removes their
interpretation as universal prerequisites for every prototype. Which controls
belong to a marked workflow, ordinary repository policy, or an optional stricter
standard must be resolved explicitly. Existing repository authority is unchanged.

## Development phases

The owner selected three development phases after Q50. This sequence supersedes
the interpretation that a complete TUI/API workflow must be the first milestone.
The phase descriptions are product direction, not a completed implementation plan.

| Phase | Work | Demonstration or integration boundary |
| --- | --- | --- |
| 1. Feature/library | Build the scoped feature files and reusable library operations; test them. | Every scoped feature must be runnable through a file or CLI command before moving to Phase 2. A direct test driver or command is sufficient; the finished ergonomic CLI is not required yet. |
| 2. CLI and Compiler | Expose primitives through a CLI; then add interactive use and improve ergonomics for future callers. Build the compiler that parses OpenWarrant documents, makes projections, and optimizes agent context. | Completion establishes the official standard-and-compiler v1.0 milestone. Exact conformance and Phase 2 exit criteria remain to be specified. |
| 3. Workflow | Build applications and workflows that call the compiler, receive its output, and act on that output. | The first integration is the Knowledge Fabric Compiler. It supplies inputs to the OpenWarrant compiler and consumes the returned output as part of a larger compilation process. |

The owner named Slack, Jira, Huey, and other applications as examples of later
workflow consumers. This does not select a required connector for every named
application or claim that those integrations already work. (C11)

The intended composition is:

**Knowledge Fabric Compiler → OpenWarrant compiler → Knowledge Fabric Compiler → consuming workflow.**

The Knowledge Fabric interface, responsibility for each input and output, and
Workflow-phase acceptance evidence remain to be designed. Its first-consumer
role does not make it necessary to exercise Phase 1 libraries or the Phase 2
standalone compiler. The exact Phase 1 feature inventory also remains open.

C12 directs consolidation of product decisions and agreed artifact/context
improvements into the SAS draft. C14 clarifies that this is refinement of SAS
1.0.0 toward stable, with the earlier baseline designated 1.0.0-rc.1, not a new
1.1 release. Original signed record identities and bytes remain preserved;
the [SAS index](../sas/drafts/README.md) explains the reclassification and pending
registry/adoption work. Workflow clients and the Knowledge Fabric Compiler
integration are later work, not v1.0 prerequisites.

## Planned workflow clients and execution adapters

- A local background service, shared API, and native TUI on Linux and macOS
  remain planned workflow surfaces. Browser UI follows the TUI through the API.
- Two agent connections remain intended: an already-connected agent using an
  OpenWarrant skill, or a coding-agent command launched by the tooling.
- LAMU, Katana, BLUT, and Liminal remain future adapters, not prerequisites for
  proving the standalone libraries and compiler. See the earlier
  [source investigation](../research/implementation-adapters.md).

The proposed OpenWarrant reference tooling supplies the work protocol and
compiler. The selected agent harness supplies its agent loop and sandbox. The
tooling checks that required execution protections are available before admitting
work. A skill or Git worktree alone does not establish isolation.
(Q12, Q14, Q17-Q18, Q32-Q33)

The interactive CLI belongs to Phase 2. Full workflow clients consume compiler
outputs in Phase 3. Knowledge Fabric Compiler is the first integration in that
phase; the remaining client schedule must follow that ordering. (Q17, Q50, C11)

## Artifact authoring and semantic compilation

The primary source developers edit and commit is one readable Markdown file
with a small structured metadata header and precisely defined sections. This
selects the normal authoring format, not every future import/export format.
Header encoding, exact fields, and section grammar remain to be specified. (Q42)

Context pointers in v1 support both explicit references and structured conditions
that the compiler evaluates without a model call. Work stage, declared subsystem,
and file path were examples in Q51; the condition vocabulary and grammar are not
yet selected. AI may help draft pointers. Dependency selection and the precise
package format remain design decisions; Q53 requires portable offline delivery.
See the
[artifact and projection review](agent-context-review.md) for observed gaps and
proposals; its recommendations are not automatically confirmed requirements.

When a referenced rule is available but a condition lacks an input needed to
decide applicability, include the rule and its dependencies and flag uncertain
applicability. Preserve the rule's conditions and the uncertainty; inclusion
does not assert that the rule applies unconditionally. This is a context-selection
fallback, not permission to proceed through missing permissions or unresolved
architecture decisions. Required-source resolution, access restrictions, conflicts,
and budget constraints still apply. (Q52)

V1 must export a portable context package containing the task brief, exact rules,
required dependencies, and source manifest. A recipient can read all required
task context without the original repository or a running OpenWarrant service.
An optional resolver may provide another access path; it is not a prerequisite
for consuming required context. Optional references remain clearly labeled and
must not be presented as included content when their bytes are absent. This
package supplies context, not the code workspace or execution environment needed
to perform the task. Package layout and optional-background delivery remain to
be specified. (Q53)

Developers choose how to invoke the standardized artifact generator:

- In the later browser client, a button opens an agent-assisted prompt.
- A configured REPL agent drafts artifacts in the TUI.
- An agent in a harness such as Claude Code or Codex uses the OpenWarrant skill
  and CLI to draft artifacts.
- An interactive CLI lets a human select a field, write its contents, repeat,
  and compile the fields into one document. Example fields include body,
  context, and relevant locations; these examples do not freeze the schema.
- A noninteractive CLI accepts explicitly supplied fields for scripted use.

Manual field authoring and agent-assisted drafting are both first-class routes.
The generator acts as a semantic compiler producing simple, parsable documents.
It assembles a large master context document and produces smaller projections
for project management and individual agents. Each agent's projection should
contain the context needed for its assigned work. C11 stages these capabilities
through tested primitives and the CLI/Compiler phase before workflow applications.
(Q36, C11)

The master context is a generated assembly of separately maintained SAS, ADRs,
Warrants, and repository sources. Edit facts at their sources, then regenerate
the master and agent projections from those exact revisions. The assembly
preserves each source's authority rather than becoming an independently edited
source of truth. (Q37)

Agent projections carry the exact wording of applicable binding requirements
with their source revisions. An AI paraphrase cannot replace those requirements.
Background context may be summarized while retaining source provenance and its
distinction from governing material. For example, a requirement for email
verification before account activation must retain that exact condition. (Q38)

Complete, valid source documents can compile into master context and task
projections without any AI/model call. AI may draft sources, propose context
links, and prepare background summaries; compilation checks and assembles the
explicit inputs reproducibly. Missing required inputs remain visible rather
than being guessed. This is a required supported path, not a prohibition on
AI-assisted tooling. (Q39)

The master document's scope (project or work outcome), supported input/output
syntaxes, and the testable meaning of a complete task projection remain open
product questions. Exact wording does not by itself establish that the compiler
selected every applicable requirement.

## Setup and preparing work

In a new repository with no governing documents, unmarked prototyping may start
under human-granted execution permissions without first approving a minimal SAS.
SAS acceptance remains required where the chosen workflow or repository policy
calls for it. Existing governing documents remain binding. (Q48)

For the earlier reviewed-workflow setup, discover existing governing documents
and draft a minimal SAS from the repository and declared goals. An authorized
human approves it during setup, before ordinary implementation under that
workflow. Grow it through governed revisions as needed; an unchanged SAS needs
no repeated acceptance for each Warrant. (Q28, qualified by Q48)

One Warrant covers one independently reviewable outcome. For example, password
reset can contain API, email, UI, and test stages under one approval. A separate
preparation agent reuses or drafts mandatory acceptance fixtures alongside the
Warrant. The approval covers both the work and those expectations. (Q1, Q6, Q20)
These preparation requirements describe the previously selected assured workflow;
C10 permits prototypes that do not meet it. Q43 allows later qualification under
final-result standards while retaining the actual history; standards requiring
pre-implementation fixtures remain unmet when that practice was not followed.

Each reviewable Warrant presents its title, description, required outcome,
explicit constraints, fixtures/checks, and relevant source documents. ADRs and
other supported documents can deepen the implementation specification. Required
behavior, architecture boundaries, permissions, and checks are binding;
implementation steps are guidance unless explicitly constrained. (Q5-Q5a)

The compiler produces a parsable work packet with the exact contract revision,
source/context basis, inputs, outputs, obligations, permissions, and limits.
Context selection or summarization must preserve required constraints and source
provenance, distinguish advice from governing material, and expose omissions.
Humans do not assemble prompts or maintain this metadata manually. (C04-C05, Q13)

## Approving and executing

Users select Warrants by title and description, inspect details as needed, then
use one **Approve & Start** action in the per-Warrant review route. C10 also
permits autonomous work under prior human-granted permissions and configuration.
Ordinary interactive approval occurs within an authenticated, unlocked session;
policy can require stronger user-presence confirmation. Approval binds exact work and
its protected expectations. (C02-C03, Q3, Q15, Q20)

Eligible agents start automatically. If none is available, show **waiting for
agent** and start when an eligible agent connects or can be launched. Approval,
waiting, actual execution, verification, and acceptance are distinct facts. (Q21)

Each Warrant gets an isolated Git worktree. Serialize writers within it;
independent Warrants can run concurrently. Connected and launched agents share
exclusive claim ownership so both routes cannot assign competing writers.
Verification uses a separate context and isolated workspace. (Q4, Q18-Q19)

The TUI reports progress, questions, blockers, verification results, and effective
acceptance mode. Agent reports are claims; protected observations and independent
verification determine what those claims establish.

If a worker disappears, preserve progress and resume or replace it within limits.
A replacement may write only after the previous writer can no longer write.
Missed heartbeats alone do not prove that. Escalate uncertain ownership or
exhausted recovery limits. (Q22)

## Questions and changes of plan

Every executing agent has a hotline. In-scope technical questions go to an AI
adviser first. Governing decisions go to the authorized decision-maker under
policy; agents can request direct human escalation. Record the question, answer,
source/evidence, respondent, and affected work. Advice grants no new authority.
(C06-C07, Q27)

Pause affected stages and their dependents while a blocking answer is pending;
independent work continues. Preserve progress so an answer can resume useful
work without repeated discovery. (Q7)

Agents may adapt implementation steps within approved outcomes and constraints.
A material change to scope, architecture, permissions, or mandatory expectations
requires an approved revision. Prepare that revision within the same Warrant,
review it through the hotline, ask further questions when needed, then update
affected contexts and resume under the approved basis. (C08-C09, Q5-Q6)

## Verification, repair, and acceptance

In the previously selected assured workflow, mandatory fixtures exist before
implementation and are protected from performer changes. Performers may add
tests; weakening mandatory expectations requires an
approved revision. Independent verification uses a separate agent context and
workspace, evaluates the approved outcome, and reruns protected checks. (Q4, Q6)

For fixable defects, return verification evidence to the performer for repair
under the same Warrant, then independently verify the result again. Users may
choose the repair count; **three cycles after initial implementation is the
fallback when no count is chosen**. Time/spend limits may stop work earlier.
Interrupted-worker recovery is tracked separately from repair cycles. (Q24-Q25)

If the performer disputes a finding, an independent verifier rechecks an
evidence-backed rebuttal. Preserve both judgments. An authorized human settles
unresolved disagreement; the performer never clears its own gate. (Q26)

When the target branch changes, update the Warrant branch within approved scope
and rerun affected checks and independent verification on the resulting candidate.
Escalate unresolved conflicts or changes to binding constraints. Evidence whose
basis changed cannot be presented as current proof. (Q23)

Manual completion acceptance remains one easy action after required review and
verification. C10 now requires actual human acceptance for signed verified/complete
Warrants and the assurance mark. Earlier Q3/Q16 automatic-acceptance choices
remain recorded, but cannot substitute for that human act. Q47 supplies the
unmarked prototype stopping point: implementation finished, unreviewed, without
a mandatory human acceptance task until qualification is sought or policy
requires review. Show the effective mode and whether the mark has been earned.

Earlier Q8-Q9 permit policy delegation of architecture amendments and SAS
acceptance, with direct human approval as the default. Reconcile the scope of
those delegated acts with C10's human-backed signed completion and assurance
requirements; do not infer that delegated authority earns the mark. Automation
may propose policy changes; only an authorized human can approve changes to
effective automation policy. (Q10)

Acceptance may merge a verified change when both Warrant and repository policy
permit. Otherwise, present accepted work ready to merge. Deployment requires
separate permission. A successful agent exit is not acceptance. (Q11)

In the bundled tooling, the default repository policy requires an assurance mark
before merging into main. An authorized human may explicitly allow unmarked
merges. That exception permits the merge without awarding the mark or weakening
its baseline. This governs the tooling's merge action, not basic document
validity. Deployment still requires separate permission. (Q49)

Allow backends without spend accounting only when policy permits. Display cost
as **unknown**, never zero or an enforced cap. Enforce available required limits;
if a mandatory limit, including a hard spend cap, cannot be enforced, refuse that
execution. An unavailable required check likewise cannot be recorded as passed.
(Q34, current UNKNOWN rule)

## History and adoption

Preserve accepted code versions, approved fixtures, decisions, and verification
evidence for the project's lifetime. Temporary work may be pruned under policy
without removing referenced evidence. Preservation requires recoverable bytes
and their provenance, not only hashes or summaries. (Q30)

A later Warrant may change a file delivered by an older Warrant. Preserve the
older accepted version and evidence; the new Warrant governs the successor with
explicit lineage. Ordinary later development must not create correction work
against every older delivery of that pathname. (Q2)

Import legacy records in their actual states, including unresolved work and
unknowns. Do not fabricate retrospective acceptance or provenance. Unrelated
closure debt does not block new work; relevant prerequisites and integrity or
authority problems still can. Existing external authority remains binding where
applicable. (Q35)

## Friction targets and proof

| Experience | Agreed target |
| --- | --- |
| Ordinary repository setup | Five to ten minutes or less |
| Advanced setup | Optional deeper discussion, below several hours; no exact hour cap selected |
| Routine Warrant administration after setup | At most 60 seconds; no manual record editing or required user shell commands |
| Human effort reporting | Separate administration, substantive review/decisions, and installation waits; also report totals |

These are targets, not measured achievements. Phase 1 proof is direct execution
and testing of every scoped feature. Phase 2 proves the CLI/compiler operations;
its exact exit criteria remain open. Phase 3 first integrates with Knowledge
Fabric Compiler. The earlier small OpenWarrant maintenance change remains a
candidate for later real workflow use, not a prerequisite for library or compiler
completion. (Q13, Q29, Q31, qualified by C11)

Workflow evidence, when those features are in scope, must exercise the flow and
its controls: both agent
connections, duplicate claims, protected-fixture tampering, queued availability,
blocking questions with independent work continuing, worker loss, bounded repair,
changed branch bases, human acceptance for the mark, policy-permitted unmarked
merges, retained historical bytes, and legacy-state preservation. Also exercise
unmarked prototypes starting without SAS acceptance, resting without mandatory
human review tasks, and later qualifying under applicable standards with their
actual history preserved. Record scope and limitations. Component tests or
record-only checks cannot substitute for an observed end-to-end workflow.

## Engineering contracts still to specify

The document records interview answers; the owner has reopened product
fundamentals and earlier answers remain revisable. Resolve the open product
questions before SAS revision planning. Later architecture work must make the
following contracts explicit:

- Phase 1 feature inventory with a runnable entry point and evidence for each
  feature; Phase 2 and Phase 3 exit criteria; the Knowledge Fabric Compiler input
  and output interface; client sequencing; exact standard/compiler conformance
  criteria for the Phase 2 v1.0 milestone selected in C12.
- Exact versioned assurance baseline and qualifying evidence; mark issuance,
  validation, and binding to the scoped result and human acceptance; later
  qualification without false process-history claims; remaining Q8-Q9 delegation
  semantics under the human-backed mark and signed-completion rules.
- Minimum document schema, precise Markdown/header grammar, extensions and other
  interchange formats, and assignment of individual requirements to standard
  conformance versus reference-tool workflows. Define master-context scope and
  projection completeness while preserving the source ownership selected in Q37.
- Local storage, immutable artifact retention/export, crash recovery, event/API
  schemas, batch atomicity, idempotency, and compatibility/version negotiation.
- Human/session authentication, protected policy state and credentials, actor
  roles, local authority, and transitions involving externally registered records.
- Supported agent/harness adapters on each OS, actual protection checks, read-only
  fixture/verifier access, exclusive writer handoff, and cancellation guarantees.
- Precise compiler source/conflict/omission rules, context invalidation, executable
  fixture preparation, and admissible evidence reuse after source changes.
- Time/spend and recovery defaults, repair/rebuttal accounting, hotline delivery
  and deduplication, and behavior when no authorized responder is available.
- Acceptance validity when an already accepted candidate changes before merging,
  retained artifact migration, and review presentation of remaining limitations.
- The real-work baseline and measurement procedure, including what evidence
  establishes the setup and per-Warrant friction targets on each supported OS.

These mechanisms are not silently selected by this document. Raise a further
product question if an engineering choice changes agreed behavior, authority,
scope, or friction. Current work is mutual product grilling and draft revision.
A governed SAS revision plan with those contracts and concrete acceptance evidence
follows only after shared-understanding review.

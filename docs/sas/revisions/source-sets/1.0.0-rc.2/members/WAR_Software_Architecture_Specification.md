+++
schema = "oh.war/document/1.0.0-rc.2"
kind = "sas"
id = "openwarrant:sas"
revision = 2
title = "OpenWarrant Software Architecture Specification"
state = "proposed"

[[dependencies]]
unit = "documents"
target = "format-contract.md#source"

[[dependencies]]
unit = "documents"
target = "format-contract.md#units"

[[dependencies]]
unit = "compilation"
target = "format-contract.md#request"

[[dependencies]]
unit = "compilation"
target = "format-contract.md#digests"

[[dependencies]]
unit = "selection"
target = "format-contract.md#pointers"

[[dependencies]]
unit = "selection"
target = "format-contract.md#conditions"

[[dependencies]]
unit = "selection"
target = "format-contract.md#selection"

[[dependencies]]
unit = "packets"
target = "format-contract.md#package"

[[dependencies]]
unit = "records"
target = "format-contract.md#records"

[[dependencies]]
unit = "assurance"
target = "format-contract.md#records"

[[dependencies]]
unit = "phases"
target = "phase-1-build-scope.md"

[[dependencies]]
unit = "documents"
target = "#context"

[[dependencies]]
unit = "contracts"
target = "#context"

[[dependencies]]
unit = "compilation"
target = "#context"

[[dependencies]]
unit = "selection"
target = "#context"

[[dependencies]]
unit = "provenance"
target = "#context"

[[dependencies]]
unit = "packets"
target = "#context"

[[dependencies]]
unit = "invalidation"
target = "#context"

[[dependencies]]
unit = "authority"
target = "#context"

[[dependencies]]
unit = "assurance"
target = "#context"

[[dependencies]]
unit = "records"
target = "#context"

[[dependencies]]
unit = "workflow"
target = "#context"

[[dependencies]]
unit = "recovery"
target = "#context"

[[dependencies]]
unit = "integrations"
target = "#context"

[[dependencies]]
unit = "phases"
target = "#context"

[[dependencies]]
unit = "adoption"
target = "#context"

[[dependencies]]
unit = "contracts"
target = "#authority"

[[dependencies]]
unit = "assurance"
target = "#authority"

[[dependencies]]
unit = "assurance"
target = "#contracts"

[[dependencies]]
unit = "assurance"
target = "#records"

[[dependencies]]
unit = "records"
target = "#assurance"

[[dependencies]]
unit = "packets"
target = "#invalidation"
+++

<!-- ow:unit outcome binding -->
## 1. Purpose and edition

**Edition: 1.0.0-rc.2. Status: consolidated release candidate, not accepted and not
an implementation or Stable release claim. Date: 2026-09-14.**

OpenWarrant gives humans and tools a shared document format for bounded work.
Its compiler turns explicit source documents into exact, useful task context.
Its optional assurance baseline distinguishes a human-accepted result supported
by independent evidence. The goal is less coordination and record keeping while
making AI-assisted development safer as teams move from experiments to production.

A Warrant describes one reviewable outcome. A password-reset feature can cover
API, email, interface, and tests without requiring four separate human approvals.
A Warrant can be a small draft before it becomes an executable plan. The format
SHALL NOT treat a valid draft as permission to execute or proof of completion.

The earlier baseline is designated **1.0.0-rc.1**. Its original acceptance record
literally says `1.0.0`; neither that record nor its source bytes are rewritten.
The owner has withdrawn the SAS 1.1.0 release line. The target is 1.0.0 Stable;
RC.2 is the current candidate. References to 1.1.0 in historical signed work
remain facts about that work, not another planned edition.
This RC.2 replaces the previous unaccepted working draft, not the configured
repository authority. See §18 for adoption and historical references.

<!-- ow:unit scope binding -->
## 2. Product boundary and conformance

OpenWarrant owns shared structure and field meanings. Other tools choose their
own drafting, planning, execution, and approval workflows. Four separate claims
SHALL be reported; none implies the others:

| Claim | What it establishes | What it does not establish |
| --- | --- | --- |
| Document validity | Source conforms to the selected grammar and field semantics | Complete context, permission, successful work |
| Compiler conformance | Declared inputs are parsed, resolved, projected, and packaged under this edition | Correctness of undeclared facts or an agent's implementation |
| Assurance qualification | One exact Warrant result meets §12 and has human acceptance | Qualification of the whole repository or another revision |
| Workflow readiness | The selected policy and execution environment admit an action | Qualification or successful completion |

The v1 release includes the document standard, libraries, CLI, and standalone
compiler. It SHALL compile complete explicit inputs without a model, OpenWarrant
service, Knowledge Fabric, Liminal, BLUT, LAMU, or Katana. The first supported
platforms are Linux and macOS. Network fetching, model calls, arbitrary source
execution, signing, and repository mutation are outside the pure compiler.

Sections 4–10 and the linked format contract define the standard/compiler.
Sections 11–13 define portable meaning for records and the optional assurance
profile; libraries inspect supplied records, not run an authority service.
Sections 14–16 specify later reference workflows. Implementing those workflows
is not a v1 compiler release prerequisite. Optional stricter profiles may add
requirements without redefining basic document validity or weakening the common
assurance mark. No client or sibling-project integration is mandatory for v1.

<!-- ow:unit context binding -->
## 3. Reading this specification

SHALL/SHALL NOT define requirements; SHOULD defines a recommendation that needs a
stated reason to depart; MAY defines an allowed choice. These words specify the
candidate, not evidence that an implementation complies.

This specification and [format contract](format-contract.md) form one normative
RC.2 source set. The [build scope](phase-1-build-scope.md) defines required release
proof. [Examples](examples/README.md) illustrate the contract; their expected
outputs are reference fixtures, not outputs claimed from a production compiler.
The [decision map](decision-map.md) explains how earlier answers were reconciled.
The [architecture proposal](context-packets.adr.md) records choices for adoption.

Numbering has been reorganized. A reference to an old section SHALL retain its
old source revision; `RC.1 §33.8` is not silently redirected to RC.2 §8. Requirement
IDs remain in §106 with explicit scope and changed meanings. The RC.1 document
is the source for legacy contracts, not a second set of RC.2 requirements.

### 3.1 Canonical definitions

The definitions below are normative. A conforming implementation SHALL preserve
these distinctions in its data and claims. Examples, synonym notes and wording
advice are guidance; different display wording alone is not a conformance failure.
Legacy records keep the meanings of their original editions (§18).

| Term | Meaning |
| --- | --- |
| Warrant | Document describing one bounded, reviewable outcome and its explicit constraints |
| SAS | Software Architecture Specification: the program's architecture document, represented by the `sas` document kind |
| ADR | Architecture Decision Record: context, decision and consequences represented by the `adr` kind; proposed text is not accepted authority |
| Source document | Human-editable Markdown and structured metadata at its owning source |
| Source unit | Explicitly identified text span inside a source document, classified as binding or background |
| Snapshot | Captured bytes, source revision, locator and digest; immutable when referenced, without freezing the live pathname |
| Contract | Exact Warrant outcome, scope, constraints, expectations and referenced basis to which an approval applies |
| Requirement | Rule imposed by a governing source within its declared scope |
| Expectation | Contract-bound acceptance-check entry with identity, scope and check digest, as defined in F9 |
| Obligation | Bounded acceptance condition evaluated by verification; its disposition must rest on applicable evidence |
| Check definition | Identified executable or review procedure and its exact expected behavior; executing it is a separate act |
| Observation record | Attributable record of an attempted check, its execution status, verdict and exact basis |
| Evidence | Recorded observation or retained supporting artifact with provenance and basis; an agent assertion alone is not an independent observation |
| Verification | Independent evaluation of an exact result against bounded obligations |
| Permission record | Supplied authority evidence for a specifically scoped action; not evidence that the action succeeded |
| Human acceptance | An authorized human's attributable acceptance of an exact result after required review; F8 calls its record kind `human-acceptance` |
| Qualification | Determination that one exact result satisfies every requirement of a stated assurance baseline |
| Assurance mark | Optional qualification record for one exact result under a versioned baseline, including the required human acceptance |
| Master context | Generated assembly of a caller-selected, captured source set |
| Projection | Role/stage selection of context from that master basis, preserving applicable exact content and inclusion reasons |
| Packet | Portable delivery of a projection: entry view, machine records, manifest and required source/evidence bytes |
| Generated view | Derived rendering or index of source records, with no independent source authority |
| Entry view | The packet's initial human/agent reading surface, `ENTRY.md`; its full rendered bytes are used for entry-budget accounting |
| Task brief | The role-specific task, constraints, expected outputs and stopping conditions carried in a packet |
| Blob | Exact retained source or evidence bytes addressed by digest inside a packet |
| Manifest | The typed inventory for its stated object; a packet manifest and a legacy Warrant manifest are different schemas |
| Pointer | Declared context reference with applicability and inclusion rules |
| Dependency | A declared requirement to include another exact target whenever the originating unit is selected |
| Conflict | A declared incompatibility between selected targets; its presence is not permission for the compiler to choose a winner |
| Blocker | Missing or unresolved required input or condition for a specified action; distinguish it from invalid document syntax |
| Correction | Attributable correction of an assertion in a record, preserving the prior assertion and its evidence |
| Successor work | Work governing a new delivered version, with explicit lineage to preserved earlier versions |
| Attestation | Attributable signed statement over an exact subject in its declared schema and digest domain |

### 3.2 Usage guidance

Use **generated view** for corpus indexes and other derived files; reserve
**projection** for selected task context and **packet** for its delivery object.
Do not merge requirement, expectation and obligation, or correction and successor
work. A legacy atom is a file, not a synonym for a source unit. Legacy resolution
is an edition-specific act, not an alias for RC.2 acceptance or qualification.
`war`, frontier, battery, plant, drafter and proposal are reference-tool vocabulary;
their existence does not add requirements to basic document compatibility.

<!-- ow:unit documents binding -->
## 4. Documents, identities, and authoring

The normal editable source SHALL be one Markdown file with a TOML metadata
header and defined units. The format contract fixes required fields, delimiters,
unit syntax, references, and extension rules. Warrant, SAS, ADR, and context
are supported source kinds. Arbitrary repository files and raw evidence may be
captured as opaque content; they do not become conforming documents by import.

The minimum Warrant SHALL contain a title, stable document identity, revision,
draft/proposal state, and outcome, scope, and context units. It needs no fixture,
SAS acceptance, stage, signature, execution record, or assurance claim to be
valid. Unknown context can be stated honestly. Missing required references block
a complete task packet, not parsing of an otherwise valid draft.

Generators SHALL derive IDs, hashes, manifests, and indexes where possible.
Routine authors SHALL NOT have to copy hashes or maintain a second pointer
catalog. Creating a local identifier does not allocate an enterprise identity.
Registered identities and source ownership remain with their issuing authority.
UUIDs from existing records SHALL be preserved; generators SHOULD use UUIDv7
for new globally shared identities. Aliases are names, not proof of ownership.

A snapshot SHALL identify both declared revision and exact bytes. Reusing a
revision label for changed draft bytes produces a different snapshot; it cannot
reuse old approval. An approved or accepted snapshot SHALL remain immutable.
Changing binding content after approval creates a successor contract revision.
Progress, suggestions, and generated status cannot amend that contract.

SAS sources describe system outcomes and boundaries. ADRs record material
architecture choices, alternatives, and rationale; acceptance remains external
to a document's self-declared state. A governing decision uses its owning
source's process. Internal implementation advice within approved constraints
does not require a new ADR. Linked documents retain their own authority.

<!-- ow:unit contracts binding -->
## 5. Warrant contracts and scope

One Warrant SHALL normally cover one independently reviewable outcome. Internal
stages divide execution; milestones group acceptance checkpoints. Child work and
successor work SHALL remain distinct: a child cites its exact parent basis;
a successor preserves earlier work and names what it replaces. Neither rewrites
its parent's rationale or an earlier result's evidence.

Required outcomes, explicit constraints, permissions, and mandatory expectations
are binding. Implementation steps are advice unless explicitly designated as
constraints. A detailed Warrant MAY include ADRs, architecture descriptions,
repository locations, fixtures, rollback instructions, and other supported
sources. A source pointer SHALL expose the referenced material's role.

Before a workflow starts work it SHALL resolve its required contract inputs:
bounded outcome/scope, source basis, required outputs and checks, permissions,
limits, blockers, and stopping conditions. This is readiness under that workflow,
not a mandatory field list for every draft. A prototype policy may admit a
lighter contract, but cannot erase existing governing constraints.

A requested material change to outcome, architecture, authority, scope, or
mandatory expectations SHALL produce a reviewable revision. A policy may delegate
such approval as §11 allows. Earlier attempts and their evidence remain bound to
the old basis. A refreshed packet is not an implicit amendment.

<!-- ow:unit compilation binding -->
## 6. Compiler architecture

Compilation SHALL consume captured inputs and return data. Its logical pipeline is:

**Parse → validate → capture/resolve → assemble → select → render/package → check.**

Capture is an explicit I/O boundary outside the pure functions. The caller supplies
source bytes, source identities, reference bindings, metadata, policy facts, role,
stage, selection inputs, budgets, and requested representation. A missing input
is reported; the compiler SHALL NOT invent one or consult a model to fill it.
Master assembly may cover a project or one task: its explicit source set defines
the boundary. There is no implicit “everything in this repository” completeness.

The result SHALL contain typed IR, source maps, diagnostics, selection reasons,
readiness facts, and requested outputs. Document validity, context completeness,
and workflow readiness SHALL be separate fields. A valid draft may compile into
a master or preview while task execution remains blocked. Failed complete-packet
requests SHALL return diagnostics without labeling partial output complete.

The implementation SHALL retain a Rust core for types, parsers, validation, and
record semantics; a compiler library for lowering, selection, canonicalization,
and projections; and a CLI for I/O and user interaction. Harness/model adapters
are optional consumers. CLI modules SHALL NOT become the only place a semantic
primitive can run. Phase 1 exposes direct library/test-driver calls; Phase 2
composes them into the supported compiler and CLI interface.

Canonical semantic JSON SHALL use RFC 8785. Source digests hash original bytes.
Structured object digests SHALL use explicitly versioned domains; old domains
and canonical bytes SHALL NOT be repurposed. IDs, clocks, host paths, network
responses, and runtime environment SHALL NOT enter reproducible content unless
supplied as explicit inputs. Identical captured inputs and versioned options
SHALL produce identical canonical output and packet file bytes.

<!-- ow:unit selection binding -->
## 7. Context selection and dependency coverage

The compiler SHALL select explicit references and evaluate structured conditions
without AI. RC.2 conditions cover stage, subsystem, and repository-relative file
paths. The format contract specifies grammar, matching, and three-valued logic.
No executable expressions, arbitrary regex engines, or implicit repository scans
are condition evaluation.

A true required pointer includes its target. A false pointer records omission.
If a valid condition lacks an input, include an available required target and
its required dependency closure, retain unknown applicability, and identify the
missing field. This fallback SHALL NOT assert that a rule's condition is true,
grant permission, settle architecture, bypass access controls, or ignore a budget.
Malformed conditions are errors, not unknown applicability.

Selection SHALL retain exact applicable binding text with declared definitions,
conditions, exceptions, and schemas. Binding meaning may have no uppercase modal
keyword. Keyword extraction can make a navigation index; it cannot prove complete
context. Authors and source review remain responsible for correct scope and
dependency declarations. Compiler completeness is relative to those declarations.

Stable unit IDs SHALL survive heading changes. Required references must resolve
to exact snapshots and spans; missing, ambiguous, stale, or mismatched targets
prevent a complete packet. Dependencies use identity plus source digest plus unit
ID. Deduplication merges identical units and retains all reasons. Distinct
revisions of the same unit are not interchangeable.

Dependency cycles containing available consistent text SHALL be included once
per unit and reported; they do not inherently prevent reading context. Traversal
SHALL terminate. Execution dependency cycles are different and block scheduling.
Conflicting required authorities, explicitly declared incompatible units, or two
required revisions of the same document identity SHALL block completeness unless
an explicit authorized reconciliation is supplied. Mere numeric priority or
order of discovery SHALL NOT silently discard a binding rule. The compiler
cannot detect arbitrary semantic contradictions in natural language; that limit
SHALL be visible.

<!-- ow:unit provenance binding -->
## 8. Authority, summaries, and source trust

A source keeps its holder, jurisdiction, trust, classification, and taint through
assembly and projection. Import, registration, a digest, and a confident sentence
do not establish that a source is authoritative. Caller-supplied authority facts
SHALL identify their policy basis and evidence; self-declared source metadata
alone cannot elevate access or grant authority.

Background MAY be summarized before compilation. Each summary SHALL retain exact
source references, derivation method, whether a model produced it, and inherited
trust, classification, and taint. Conflicting or incomparable access labels require
explicit policy treatment; absent permission blocks disclosure. No summary may
replace binding text or hide its dependencies. Untrusted text remains data; it
cannot override instructions merely because a compiler packaged it.

Each task entry SHALL show outcome, scope, required outputs, blockers, and
observable stopping conditions. Reference material SHOULD sit beside the rule it
explains or behind a precise pointer. Relevant glossary definitions travel with
the units that need them; unrelated glossary text need not load by default.
Role-specific preparation, implementation, verification, and human-review views
SHALL preserve common constraints and the source of every view-specific fact.
A performer's desired verdict SHALL NOT be presented as independent evidence.

<!-- ow:unit packets binding -->
## 9. Portable context packets and budgets

A packet SHALL contain four logical parts: task brief, exact binding context,
reference catalog, and integrity-bound manifest. RC.2 uses a directory package
with `ENTRY.md`, `packet.json`, `manifest.json`, and digest-addressed blobs.
The format contract defines their contents and digest domains. A resolver is
optional and has no mandatory service protocol in v1.

Every required context byte SHALL be readable using only the package, with the
original repository unavailable and no service or network. Full required evidence
SHALL be accessible even when a view displays a labeled excerpt. Optional absent
content SHALL be described as a reference, not supplied evidence. A packet does
not provide an executable workspace, sandbox, human approval, or source authority.

The canonical entry view SHALL include the task brief and every selected binding
unit; optional background defaults to references only. Required dependency text
cannot be hidden behind a remote resolver. Included raw source blobs preserve
exact provenance; source-level access applies to the entire blob. If a source
contains material that cannot be disclosed to the recipient, compilation SHALL
refuse that package rather than leak it. Authors can split it into separately
controlled sources before capturing a new basis.

Measure package bytes, entry bytes, estimated tokens, and observed harness tokens
separately. The compiler estimate SHALL count all entry rendering overhead using
the declared method. Required content SHALL NOT be discarded to fit a budget.
Excess yields a named refusal; remedies include less optional background, a
smaller authorized task, or a larger permitted budget. The harness additionally
accounts for its system prompt, tools, history, and later retrieval. A compiler
estimate SHALL NOT be presented as enforcement of the whole model window.

<!-- ow:unit invalidation binding -->
## 10. Invalidation, preservation, and safe boundaries

The cache key SHALL cover all captured sources and routing declarations, including
omitted candidates, dependencies, source/authority policy, role/stage, scope
inputs, compiler/schema versions, representation, and budgets. An omitted source
becoming applicable SHALL invalidate its old selection. Changed actual scope
requires re-evaluation by the consuming workflow; old packets keep their old basis.
A cache hit is neither permission nor fresh verification.

Readers SHALL check package digests, required membership, schema, source spans,
and content references before reporting integrity. A digest establishes consistency
with the supplied package, not publisher authenticity. External acceptance must
bind the package/root digest to a trusted actor. Import SHALL reject path escape,
unsafe file types, symlinks, duplicate normalized paths, oversized inputs, invalid
UTF-8 for text, and unsupported required formats without partial success claims.

Parsing and packaging SHALL not execute embedded code, commands, or links.
Network resolution and process execution are explicit caller actions. Source
capture and output publication SHALL be atomic with respect to their snapshot
checks: refuse a changed source or occupied destination instead of mixing bases.
Old accepted bytes, fixture versions, evidence, and decisions SHALL remain
recoverable for the project lifetime by default. Hash-only retention is insufficient.

<!-- ow:unit authority binding -->
## 11. Permission and acceptance semantics

Execution permission, automated policy disposition, governance adoption, human
acceptance, merging, and deployment are distinct acts. Every act SHALL bind its
exact subject, actor kind, acting role, policy revision, decision, and evidence.
A source field stating `accepted` is not an acceptance record. Libraries can check
record structure and supplied bindings; authenticity requires a trusted verifier.

Human-granted policy MAY permit agents to start work without fresh per-Warrant
approval, including prototypes with no prior SAS in a repository without governing
documents. Existing governing documents remain binding. A named repository policy
may automate result disposition or delegate architecture amendments and SAS
adoption. Direct human approval is the default for those governance acts. Delegated
SAS adoption SHALL be labeled a delegated governance act, not human acceptance of
the Warrant that produced it and not assurance qualification. This reconciles
Q8–Q9 with C10: delegation can change an authoritative SAS where local authority
permits, but cannot fabricate a human review or signed Warrant completion.

Automation MAY draft changes to its effective permission policy; only an authorized
human SHALL approve those changes. This restriction includes indirect policy
changes through a SAS, parent policy, adapter configuration, or delegation chain.
An agent cannot expand its own effective authority through a delegated act.

A human may enable a named automatic mode; its effective revision and limits
SHALL remain visible. Individual Warrants may require manual review. Automatic
handling can end at `implementation_finished` or a policy-disposition record;
it SHALL NOT call itself human acceptance, signed completion, or the assurance
mark. Unreviewed prototypes need no mandatory human acceptance task until requested
or required by policy. Their real state remains visible.

<!-- ow:unit assurance binding -->
## 12. Optional assurance baseline

The provisional profile identifier is `oh.war/assurance-baseline/1.0.0-rc.2`.
The public mark name remains a branding choice, not a build dependency. Repositories
MAY strengthen this profile but SHALL NOT weaken it while claiming the same mark.
Qualification covers one Warrant result, exact code revision, contract, and scope.

All baseline conditions SHALL be satisfied:

1. The result has a bounded outcome and scope, exact source/contract basis, and
   permitted work history. Missing or incompatible authority remains unresolved.
2. Each acceptance obligation names its scope and admissible evidence. Requirements,
   fixtures, check definitions, and actual executions are separate records.
3. Mandatory expectations are frozen before the qualifying verification run and
   protected from the performer during that run. A separate verifier reruns
   required protected checks on the exact candidate in an isolated workspace.
4. Independent verification evaluates behavior and technical changes against the
   contract, records findings and evidence, and cannot be supplied by the performer
   under another label. Different context and workspace are required; credential,
   model, and organization separation may be strengthened by repository policy.
5. Required checks have observed passing results. Unavailable observation is
   `UNKNOWN`, observed violation is `FAIL`, and infrastructure errors are recorded
   separately from verdict. Required unknown or failed checks block qualification.
6. Adequacy evidence includes relevant refusal/control cases; test success alone
   does not establish that a check can detect the prohibited behavior. Claims are
   bounded by what was exercised. Unresolved implementation defects block; residual
   risks must be explicit and accepted by an authorized human, not used to relabel
   a failed mandatory requirement as passing.
7. A human reviews outcome, verification findings, and remaining risks, inspects
   code as needed, then explicitly accepts this exact result. This does not claim
   line-by-line human review unless that actually occurred.
8. The qualification record binds the profile revision, Warrant/contract, code and
   artifact digests, checks, independent verification, human acceptance, scope,
   limitations, and recoverable supporting evidence. Its authenticity is checkable
   under the repository's declared trust policy.

Later qualification is allowed. Fixtures added after prototyping may satisfy this
final-result baseline when protected for verification; they SHALL NOT establish
that tests existed before implementation. A stricter `fixtures-before-work`
profile requires preparation and joint approval of fixtures and Warrant before
execution. History never gets retroactive process claims.

Changing the accepted candidate, required expectations, or relevant verification
basis requires new verification and acceptance for the new result. The earlier
mark remains historical at its old revision. Dispute, invalidated evidence, or
annulment SHALL be recorded as standing, without deleting the earlier fact.
One marked Warrant SHALL NOT qualify a whole repository or release.

<!-- ow:unit records binding -->
## 13. Records, history, and successor work

The v1 compiler SHALL inspect supplied record facts through the small record
interface in the format contract. It SHALL preserve claim, observation, inference,
judgment, verification, acceptance, and qualification as distinct classes.
It does not mint authoritative evidence by parsing an agent-authored report.
Record-only validation reports its trust and runtime limitations explicitly.

Work state SHALL separate lifecycle phase, execution condition, result,
currency, and acceptance/qualification standing. `implementation_finished` means
the performer reports implementation done; `verified` requires independent
verification; signed `complete` requires human acceptance. Superseded, disputed,
annulled, unknown, and unreviewed are not interchangeable. Claims remain claims.

A successor Warrant MAY change a file delivered by an older resolved Warrant.
Preserve the older bytes and evidence; the new Warrant governs the new version
with explicit lineage. Normal successor work SHALL NOT create correction work
against every older Warrant mentioning the pathname. Correcting an old record's
assertion is a separate attributable act and does not overwrite old evidence.

Legacy imports SHALL preserve actual states, identity, source holders, timestamps,
and signature subjects. No fabricated historic fixtures, acceptance, or receipts.
Unresolved unrelated legacy work SHALL NOT block new work; relevant dependencies,
authority defects, or integrity defects still may. Migration from current live-file
pins SHALL first preserve and verify the old artifact bytes; it cannot simply
stop checking pins and call missing history preserved.

<!-- ow:unit workflow binding -->
## 14. Reference workflow: preparation, execution, and questions

This section defines Phase 3 behavior. It is not a compiler-side agent scheduler.

The reviewed path SHALL be prepare → approve/start → execute/questions → independent
verification → human acceptance. A separate preparation agent reuses or drafts
fixtures. The human reviews the Warrant and those expectations together. Users
see title and description, inspect detail as needed, and perform one approval
act bound to exact work. Ordinary approval uses an authenticated, unlocked app
session; stronger confirmation is policy-controlled.

The local service and shared API SHALL support a connected agent using an
OpenWarrant skill and a configured coding-agent command launched by the service.
If no eligible agent is available, show waiting-for-agent and start when one is
available. A claim is not execution; execution is not acceptance.

Each Warrant SHALL have one isolated Git worktree, with writers serialized within
it. Independent Warrants may run concurrently. A verifier uses separate context
and workspace. The harness supplies sandbox enforcement; OpenWarrant checks the
required protections. Neither a skill nor a worktree alone is a sandbox.

Every executing agent SHALL have a hotline. In-scope technical questions go to an
AI adviser first; governing questions go to an authorized decision-maker. Agents
may request direct human escalation. Advice is not a grant of authority. Store
question, exact basis, respondent, answer/evidence, and affected stages. Pending
blocking answers pause affected work and dependents, while independent work
continues. No eligible responder means visible waiting, not guessed permission.

<!-- ow:unit recovery binding -->
## 15. Reference workflow: recovery, review, and integration

Preserve worker progress. Automatic resume or replacement SHALL stay within
human-controlled retry, time, and spend limits. A replacement may write only
when the previous writer can no longer write; heartbeat expiry alone is not
proof. Uncertain ownership or exhausted limits requires escalation.

Fixable implementation defects MAY be repaired under the same approved Warrant
and then independently verified again. The user selects repair count; three
cycles after initial implementation is the fallback only when unspecified.
Worker recovery and evidence-backed rebuttal review are counted separately.
Policy SHALL define finite recovery/time limits before unattended recovery; this
SAS does not invent a universal dollar budget. Unknown cost is allowed only when
policy permits and is shown as unknown, never zero. A mandatory hard spend cap
without usable accounting/enforcement refuses execution.

On dispute, an independent verifier rechecks an evidence-backed rebuttal. A human
settles unresolved disagreement. The performer cannot clear its own gate. When
the target branch changes, update within approved scope, rerun affected checks
and verification, and escalate conflicts or changed binding requirements. Evidence
from an older basis cannot stand in for verification of the changed candidate.

Acceptance may merge only where Warrant and repository policy explicitly permit.
Otherwise accepted work remains ready to merge. Bundled tooling SHALL require
an assurance mark before merging to main by default. An authorized human may
allow unmarked merges; the exception never awards the mark. Deployment needs
separate permission. External Git clients are outside this enforcement claim.

Batch human approval MAY cover explicitly listed exact subjects after review.
Validate every subject, actor role, and digest before committing the batch;
a changed member invalidates the whole approval request. Retry must be idempotent.
A UI presents the request; the trusted act boundary attributes the user's action.
Authenticated-session approval does not mean a rendering itself becomes an
independent authority. Detailed transaction/storage and signature transport belong
to the Phase 3 design, before implementing these acts.

<!-- ow:unit integrations binding -->
## 16. Integrations and developer friction

Phase 3 SHALL first integrate the Knowledge Fabric Compiler: it supplies explicit
inputs to OpenWarrant, receives compiler output, then continues its larger process.
OpenWarrant SHALL retain a standalone interface; the larger compiler does not
become an implicit dependency. Katana may supply execution, LAMU context/model
services, BLUT DAG orchestration, and Liminal source compilation through later
adapters. Native systems retain their artifact and authority ownership.

Local service/API and native TUI precede browser UI. Slack, Jira, Huey, and GitHub
Issues are possible consumers, not required connectors. Repository policy controls
which system owns a governing act. Federation cannot erase local source authority.

Ordinary setup targets 5–10 minutes or less. Advanced setup may involve longer
optional discussion below several hours; no exact hour ceiling was chosen.
After setup, routine administration SHALL require no manual metadata edits or
mandatory user shell commands and target at most 60 seconds per Warrant. Measure
review/decision time, administration, downloads/waits, and total time separately.
These workflow targets are not current measured results or Phase 1 CLI limits.
A small meaningful OpenWarrant maintenance change is the eventual full-workflow
proof; clearing all old Warrants is not its prerequisite.

<!-- ow:unit phases binding -->
## 17. Build phases and release criteria

1. **Feature/library:** implement and test the scoped primitives in the build
   scope. Every feature has a direct runnable driver, positive/refusal/boundary
   cases, and observable outputs. The ergonomic CLI is not required yet.
2. **CLI and Compiler:** compose the primitives into supported standalone commands,
   noninteractive field authoring, interactive field editing, source capture,
   deterministic master/projection generation, offline export and checking.
   Prove the published contract and release criteria before declaring v1.0 Stable.
3. **Workflow:** integrate Knowledge Fabric Compiler first, then service/clients,
   agent execution, hotline, admission, recovery, approval, and merge workflows.

The [build scope](phase-1-build-scope.md) is the exact Phase 1 inventory and
acceptance matrix. Its tests are planned until run against production code.
Library demonstrations SHALL NOT be called workflow proof; example validation
SHALL NOT be called compiler conformance. Phase 2 requires all in-scope conformance
cases passing on Linux/macOS, deterministic output on repeated runs, offline packet
consumption, no source mutation, and successful authoring through both CLI modes.
No pending required check may be hidden behind an aggregate green result.

Context optimization SHALL preserve declared binding coverage and be measured.
Compare full-source, current keyword extraction, and new packets on fixed task
sets. Deterministic correctness and safe refusal are release gates; stochastic
agent efficiency measurements are bounded experiments. Report omissions, wrong
actions, task completion, retrieval, bytes/tokens, elapsed time, and human help.
Do not claim improvement from a single shorter prompt or a hand-selected run.

<!-- ow:unit adoption binding -->
## 18. Compatibility, adoption, and governance

The existing `oh.war/atom/v1`, canonical IR, receipts, signatures, and digest
domains retain their meanings. RC.2 adds a distinct document adapter and packet
schema. Explicit legacy import may map whole old atom files to one or more source
units while preserving original canonical objects; it SHALL NOT pretend an old signature accepted new RC.2 bytes.
Unsupported legacy meaning is retained with diagnostics, never silently omitted
from a supposedly complete projection. No live corpus rewrite is required merely
to author RC.2 examples.

The accepted RC.1 source and original registry entry remain unchanged. Adoption
requires an architecture decision, an exact source-set manifest, and the existing
authority's acceptance process. This drafting task creates no authorization,
verification disposition, signature, enterprise ID, or release approval.
RC.2 source-set acceptance SHALL include this file and the format/build contracts
by digest, so editing a linked normative document cannot silently change accepted
meaning. Example fixtures and their expected outputs must be pinned for conformance.

OW-ADR-0002 and OW-ADR-0003 continue to govern the legacy parsers. The RC.2 TOML
adapter is a distinct proposed architecture choice. OW-ADR-0001's JCS choice is
retained. OW-ADR-0012's current correction workflow remains binding for current
records until a migration is adopted. At inspected base `f22ef2f`, OW-WAR-0071,
OW-WAR-0072 and OW-WAR-0073 each have a human authorization record dated
2026-09-13; their bodies describe the withdrawn 1.1.0/batch/dashboard plan.
Authorization of that work is not acceptance of the unaccepted 1.1.0 SAS.
OW-ADR-0019 still declares acceptance contingent on that SAS acceptance.
OW-ADR-0020 still has `status: proposed` metadata while its prose acceptance
condition names authorization of OW-WAR-0073, which exists. Preserve these exact
records and surface their status distinction; this draft does not rewrite their
metadata, issue an acceptance or decide away the historical inconsistency.

§15 retains exact batch subjects while allowing the subsequently selected
authenticated-session approval route. The owner selected preservation plus successor work when needed for the signed
legacy Warrants. Their actual supersession is a separate governed act; no
historical contract is amended by this candidate. Reuse on the RC.2 line
requires the explicit approved successor basis. Runtime/receipt
adapters retain existing protocol commitments where selected.

Legacy import SHALL report the original resolution, subject, signer, verdict and
provenance as legacy facts. A legacy `resolved` state alone SHALL NOT create an
RC.2 `human-acceptance` or `qualification` record. Any derived acceptance claim
requires authenticated evidence for the same exact result and the applicable
review meaning; missing evidence yields unknown or ineligible with reasons.
Qualification is evaluated separately against every RC.2 baseline requirement.
Do not infer a mark, stronger fixture history or new signed completion from an
old resolution or a note saying the earlier edition bundled those concepts.

The Phase 2 `war document` and `war context` surfaces are additive. Existing
legacy commands retain their meanings; aliasing `war compile` to RC.2 semantics
without an explicit migration is prohibited. New guidance and context packets
SHOULD label legacy glossaries and instructions by edition. Current pinned
glossaries are changed only through their applicable governed process.

Historical section references, requirement titles, and draft decisions are mapped
in the decision map. The earlier specification is retained for interpreting its
own records. Once adopted, RC.2 conformance is determined by this source set and
its scoped requirements, not an unqualified union with every legacy SHALL.

<!-- ow:unit requirements binding -->
## 106. Normative requirement index

The index preserves existing IDs for traceability. Titles below describe their
RC.2 meaning, not a claim that historical contracts used that meaning. `D` means
document/compiler, `A` optional assurance records, `W` later workflow, and `L`
legacy/federated compatibility. Scope is explicit; W/L integration completion is
not a prerequisite for the standalone compiler. The decision map records major
supersessions. Source clauses, not titles alone, define full requirements.

| ID | Requirement | Scope | Source section | Build coverage |
| --- | --- | --- | --- | --- |
| WAR-SAS-RQ-001 | Document identity is stable; legacy UUIDs are preserved and new shared identities should use UUIDv7 | D/L | 4, 18 | F03/F10 |
| WAR-SAS-RQ-002 | Local aliases do not substitute for source identity or authority | D/L | 4, 18 | F03/F10 |
| WAR-SAS-RQ-003 | Selected federation authorities allocate their official enterprise identity | D/L | 4, 18 | F03/F10 |
| WAR-SAS-RQ-004 | Registration and import do not silently transfer source ownership | D/L | 4, 18 | F03/F10 |
| WAR-SAS-RQ-005 | Federated cross-repository relations retain their issuing and local authorities | D/L | 4, 18 | F03/F10 |
| WAR-SAS-RQ-010 | New documents and legacy authored atoms remain editable sources before immutable approval | D | 4, 6 | F01–F03/F06 |
| WAR-SAS-RQ-011 | Bound source content changes through its owning authority | D | 4, 6 | F01–F03/F06 |
| WAR-SAS-RQ-012 | Generated views are regenerated from sources rather than edited as authorities | D | 4, 6 | F01–F03/F06 |
| WAR-SAS-RQ-013 | Document composition is typed, ordered, and deterministic | D | 4, 6 | F01–F03/F06 |
| WAR-SAS-RQ-014 | Human and canonical machine views derive from one captured source basis | D | 4, 6 | F01–F03/F06 |
| WAR-SAS-RQ-015 | Missing required source content prevents a complete task packet | D | 4, 6 | F01–F03/F06 |
| WAR-SAS-RQ-020 | Material governing architecture decisions remain first-class ADRs; in-scope advice is not a new decision | D/W | 4–5, 13 | F02/F09/F10 |
| WAR-SAS-RQ-021 | Decision indexes are generated from source records | D/W | 4–5, 13 | F02/F09/F10 |
| WAR-SAS-RQ-022 | Work traces to applicable SAS requirements; no SAS or roadmap is required for every valid prototype draft | D/W | 4–5, 13 | F02/F09/F10 |
| WAR-SAS-RQ-023 | Child Warrants cite their exact parent basis | D/W | 4–5, 13 | F02/F09/F10 |
| WAR-SAS-RQ-024 | Children do not rewrite parent rationale | D/W | 4–5, 13 | F02/F09/F10 |
| WAR-SAS-RQ-025 | Successor work preserves earlier Warrants and their historical standing | D/W | 4–5, 13 | F02/F09/F10 |
| WAR-SAS-RQ-030 | Approved contract snapshots are immutable | D/A/W | 5, 11, 13 | F03/F09/F10 |
| WAR-SAS-RQ-031 | Progress cannot amend a contract | D/A/W | 5, 11, 13 | F03/F09/F10 |
| WAR-SAS-RQ-032 | State separates lifecycle, execution condition, result, currency, and acceptance or qualification standing | D/A/W | 5, 11, 13 | F03/F09/F10 |
| WAR-SAS-RQ-033 | Material amendment creates a new contract revision | D/A/W | 5, 11, 13 | F03/F09/F10 |
| WAR-SAS-RQ-034 | Prior attempts retain their original basis | D/A/W | 5, 11, 13 | F03/F09/F10 |
| WAR-SAS-RQ-035 | Workflow readiness checks its required inputs; document validity is separate | D/A/W | 5, 11, 13 | F03/F09/F10 |
| WAR-SAS-RQ-036 | Successor work preserves old delivered bytes and evidence without freezing a live pathname; corrections remain separate | D/A/W | 5, 11, 13 | F03/F09/F10 |
| WAR-SAS-RQ-040 | Milestones and execution stages are distinct | W | 5, 9, 14–15 | Phase 3 |
| WAR-SAS-RQ-041 | Typed stage ports remain a selected orchestration adapter contract | W | 5, 9, 14–15 | Phase 3 |
| WAR-SAS-RQ-042 | An executing actor receives an exact task dispatch basis | W | 5, 9, 14–15 | Phase 3 |
| WAR-SAS-RQ-043 | Task context exposes basis, required outputs, constraints, limits, and stopping conditions | D/W | 5, 9, 14–15 | F06/F08/F09 |
| WAR-SAS-RQ-044 | Execution authority is explicit and bounded by human-approved effective policy | D/W | 5, 9, 14–15 | F06/F08/F09 |
| WAR-SAS-RQ-045 | Replay, repair, and worker recovery retain distinct attempt history and limits | W | 5, 9, 14–15 | Phase 3 |
| WAR-SAS-RQ-046 | Entry token estimates include rendering overhead; exceeding a required-content budget is a refusal | D/W | 5, 9, 14–15 | F06/F08/F09 |
| WAR-SAS-RQ-050 | Assurance qualification decomposes completion claims into bounded obligations | A | 12–13 | F09 |
| WAR-SAS-RQ-051 | Claims declare scope bounded by their evidence | A | 12–13 | F09 |
| WAR-SAS-RQ-052 | Claim, evidence, observation, inference, judgment, acceptance, and qualification remain distinct | A | 12–13 | F09 |
| WAR-SAS-RQ-053 | Performer reports cannot satisfy independent verification | A | 12–13 | F09 |
| WAR-SAS-RQ-054 | Required unknown or failed checks block assurance qualification | A | 12–13 | F09 |
| WAR-SAS-RQ-055 | Assurance requires adequacy evidence with applicable refusal and control cases | A | 12–13 | F09 |
| WAR-SAS-RQ-056 | Required check definitions and expectations are versioned and protected from performer changes during verification | A | 12–13 | F09 |
| WAR-SAS-RQ-057 | Invalidated evidence changes dependent qualification standing without rewriting history | A | 12–13 | F09 |
| WAR-SAS-RQ-058 | Residual risk requires explicit authorized human acceptance and cannot waive a failed baseline requirement | A | 12–13 | F09 |
| WAR-SAS-RQ-059 | Human acceptance binds exact contract, result, and assurance evidence | A | 12–13 | F09 |
| WAR-SAS-RQ-060 | Knowledge Fabric owns authority and lifecycle for its selected federated workflow, not every standalone compiler use | W/L | 2, 16, 18 | Phase 3 |
| WAR-SAS-RQ-061 | OpenWarrant owns standalone standard and context compilation; Liminal is an optional adapter | D | 2, 16, 18 | F03/F10 |
| WAR-SAS-RQ-062 | Selected Katana adapters retain Katana runtime and PromptIR ownership | W/L | 2, 16, 18 | Phase 3 |
| WAR-SAS-RQ-063 | Selected BLUT adapters retain BLUT typed execution ownership | W/L | 2, 16, 18 | Phase 3 |
| WAR-SAS-RQ-064 | OpenWarrant compiler does not require or duplicate sibling runtime kernels | D | 2, 16, 18 | F03/F10 |
| WAR-SAS-RQ-065 | Native systems retain their artifact and source authority | W/L | 2, 16, 18 | F03/F10 |
| WAR-SAS-RQ-070 | Reference authoring and compilation work offline without a model | D | 4, 6, 16–17 | F02/F11 + Phase 2 |
| WAR-SAS-RQ-071 | Agent-assisted planning returns reviewable structured source proposals | W | 4, 6, 16–17 | Phase 3 |
| WAR-SAS-RQ-072 | Proposals are validated before source writes and cannot grant themselves authority | D | 4, 6, 16–17 | F02/F11 + Phase 2 |
| WAR-SAS-RQ-073 | Planning identifies material architecture decisions for ADR proposals | W | 4, 6, 16–17 | Phase 3 |
| WAR-SAS-RQ-074 | Document and compiler checks are deterministic and agent-free | D | 4, 6, 16–17 | F02/F11 + Phase 2 |
| WAR-SAS-RQ-075 | Generated views are checked against their exact source basis | D | 4, 6, 16–17 | F02/F11 + Phase 2 |
| WAR-SAS-RQ-076 | Federated controlled actions use the owning authority interface rather than direct status edits | W | 4, 6, 16–17 | Phase 3 |
| WAR-SAS-RQ-077 | Selected code, document, and run workflows each require their own observed gates and examples | W | 4, 6, 16–17 | Phase 3 |
| WAR-SAS-RQ-080 | Canonical structured preservation uses RFC 8785 JSON | D/L | 6, 10, 13, 18 | F07/F10 |
| WAR-SAS-RQ-081 | Digests use explicit algorithms and versioned domains; original source bytes remain recoverable | D/L | 6, 10, 13, 18 | F07/F10 |
| WAR-SAS-RQ-082 | Preservation exports retain contract, sources, receipts, assurance and acceptance when present without inventing missing history | D/L | 6, 10, 13, 18 | F07/F10 |
| WAR-SAS-RQ-083 | Export-import-export preserves semantic identity and original evidence bytes | D/L | 6, 10, 13, 18 | F07/F10 |
| WAR-SAS-RQ-084 | Historical superseded, disputed and annulled records remain available | D/L | 6, 10, 13, 18 | F07/F10 |
| WAR-SAS-RQ-085 | Existing SSH-signed acts retain their portable attestations and original subjects | D/L | 6, 10, 13, 18 | F07/F10 |
| WAR-SAS-RQ-090 | Master context is a generated assembly of exact source revisions; source authority remains at its origin | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-091 | Binding units retain exact text with required definitions, conditions, exceptions and schemas | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-092 | Context pointers have stable targets, explicit references and versioned structured conditions | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-093 | Unknown applicability includes an available rule and its dependencies while preserving uncertainty and authority blockers | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-094 | Required dependency coverage is explicit, terminating and provenance-preserving; inconsistent or unresolved required meaning cannot be complete | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-095 | Background summaries preserve source provenance, trust, classification and taint and never replace binding text | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-096 | Complete explicit inputs compile reproducibly without a model call or mandatory sibling runtime | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-097 | Agent packets contain a task brief, binding context, reference catalog and integrity-bound source manifest | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-098 | V1 exports all required task context for offline reading without the original repository or running service | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-099 | Optional resolvers and absent optional references remain distinct from required bytes actually supplied | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-100 | Role-specific packets preserve common requirements, independence and access to full required evidence | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-101 | Budget accounting covers compiler rendering overhead and distinguishes estimates from observed harness usage | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-102 | Required context is never discarded to satisfy a budget | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-103 | Source, scope, selection, policy and representation changes invalidate affected projections without rewriting earlier packets | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-104 | Stable source-unit identities survive heading changes and retain exact source locators | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-105 | Task instructions expose bounded outputs and observable completion criteria with supporting material progressively disclosed | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-106 | Generators derive indexes and digests without requiring manual duplicate metadata maintenance | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-107 | Context optimization claims are bounded by repeated task evaluation including omissions, wrong actions, tokens and human effort | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-108 | OpenWarrant 1.0.0 Stable follows library and CLI/compiler conformance; workflow integrations follow afterward | D | 6–10, 17 | F03–F08; Phase 2 |
| WAR-SAS-RQ-110 | Minimal Markdown documents are valid independently of workflow readiness | D | 2, 4 | F01/F02 |
| WAR-SAS-RQ-111 | Structured source grammar and extension interpretation are versioned and reject unsupported required semantics | D | 4; F1–F4 | F01/F02/F04 |
| WAR-SAS-RQ-112 | A named policy may automate work or governance adoption but never fabricate human acceptance or assurance | A/W | 11 | F09; Phase 3 |
| WAR-SAS-RQ-113 | Only an authorized human approves changes to effective automation policy, including indirect changes | A/W | 11 | F09; Phase 3 |
| WAR-SAS-RQ-114 | Common assurance requires independent evidence and human acceptance for one exact Warrant result | A | 12 | F09 |
| WAR-SAS-RQ-115 | Later qualification preserves actual fixture timing and does not claim fixtures-before-work retrospectively | A | 12 | F09 |
| WAR-SAS-RQ-116 | Unreviewed prototype results may rest without mandatory acceptance tasks | W | 11, 13 | Phase 3 |
| WAR-SAS-RQ-117 | Default main-branch merge requires assurance; human policy may permit unmarked merges; deployment is separate | W | 15 | Phase 3 |
| WAR-SAS-RQ-118 | Writers serialize per Warrant worktree and replacement requires proof prior writer cannot write | W | 14–15 | Phase 3 |
| WAR-SAS-RQ-119 | Blocking hotline questions pause affected work and dependents while independent work continues | W | 14 | Phase 3 |
| WAR-SAS-RQ-120 | Repair count is user-selectable with three cycles as fallback; unknown cost never implies an enforced cap | W | 15 | Phase 3 |
| WAR-SAS-RQ-121 | Phase 1 exposes and tests every scoped primitive before supported compiler and CLI composition | D | 17 | F01–F11 |

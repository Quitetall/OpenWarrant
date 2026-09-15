<!-- ow:unit outcome binding -->
# OpenWarrant Software Architecture Specification

## 1. Purpose and edition

**Edition: 1.0.0-rc.3. Status: successor draft for review, not signed adoption,
implementation qualification, or Stable release. Date: 2026-09-14.**

OpenWarrant defines a shared document standard and SDK for bounded work. It reduces
coordination and record keeping while supporting progressively stronger assurance,
from agent prototypes to reviewed production workflows. A Warrant normally covers
one reviewable outcome; internal implementation stages need not become separate
human approvals. Any human or agent MAY execute a Warrant as unverified work
without a universal OpenWarrant approval or signing requirement. A Warrant MAY
explicitly require verified preparation, prerequisite qualification or signoff
before a named action; that gate SHALL be honored. Otherwise a prompt is sufficient
to request work. A valid draft SHALL NOT imply success, qualification
or access to resources the executing harness does not permit.

The OpenWarrant SDK makes standard documents and records easy to create, parse,
validate and inspect. LAMU and other semantic compilers consume the SDK to assemble
cross-document context. Workflow applications use these interfaces to coordinate
work and establish attributable acts. Semantic compiler implementation is outside
OpenWarrant's product scope; protocol meanings and conformance remain in the standard.

This draft records the owner's SDK ownership, four-phase roadmap, optional
verification and policy-controlled agent-signing decisions. It supersedes RC.2's
prospective product plan only when adopted. RC.2's captured source set and signed
OW-WAR-0074 contract remain unchanged. The historical record literally named
`1.0.0` remains the owner's designated RC.1, with its original bytes and signature.
SAS 1.1.0 remains withdrawn. Stable 1.0 is a future qualified release, not this draft.

<!-- ow:unit scope binding -->
## 2. Product boundary and conformance

| Owner | Responsibility |
| --- | --- |
| OpenWarrant standard | Grammar, field meanings, record/attestation semantics, context exchange profiles, versioning and conformance fixtures |
| OpenWarrant SDK and CLI | Single-document parsing/authoring/validation, exact source units, canonical codecs/digests, record checks, protocol types and typed adapter entry points |
| LAMU or another semantic compiler | Source acquisition/capture, cross-document resolution and dependency closure, context selection, master assembly, projections, budgets/cache and package construction |
| Workflow application | Storage, identity/policy administration, execution admission, orchestration, questions, review, signing transports, qualification issuance, merge/deploy and UI |

OpenWarrant SHALL supply a reusable SDK that runs without LAMU, a database,
network, model or workflow service. The first supported platforms are Linux and
macOS. A consumer SHALL be able to read a title, author a document or validate
supplied data without installing an inference stack. Pure SDK functions SHALL
NOT launch an agent, fetch a source, sign an act, or mutate a repository.

OpenWarrant MAY ship a small reference adapter to demonstrate compiler integration.
It SHALL exercise the SDK and a declared backend interface, not become a second
production semantic compiler. A fake backend proves transport only. Legacy compiler
commands retain their original contracts until explicitly migrated (§18).

Separate claims SHALL remain distinguishable: document validity, SDK conformance,
external compiler-profile conformance, workflow readiness, implementation completion,
independent verification findings, and optional human-backed assurance qualification.
None implies the others. A workflow may impose stricter policy without making its
rules universal requirements for basic document validity or unmarked prototyping.

Sections 7–10 specify the external context profile's required behavior. They are
requirements on providers claiming that profile, not an OpenWarrant compiler build
assignment. Phase 2 qualifies those integrations. Phase 3 supplies a real reference
webapp and first-party workflow integrations; the standard does not require every
third-party implementation to use those applications.

<!-- ow:unit context binding -->
## 3. Reading this specification

SHALL/SHALL NOT define requirements; SHOULD defines a recommendation that needs a
stated reason to depart; MAY defines an allowed choice. These words specify the
candidate, not evidence that an implementation complies.

This specification and its normative companions form one RC.3 source set. Read the
[format contract](format-contract.md) when authoring or parsing documents,
[SDK contract](sdk-contract.md) when implementing library or adapter functions,
[context views and shared work](context-views-and-shared-work.md) when selecting
agent context or coordinating repositories, and [work-stop contract](work-stop-contract.md)
when stopping, changing or resuming work. The [build scope](phase-1-build-scope.md) defines required release
proof. [Examples](examples/README.md) illustrate the contract; their expected
outputs are reference fixtures, not outputs claimed from a production compiler.
The [decision map](decision-map.md) explains how earlier answers were reconciled.
The [architecture proposal](sdk-ownership.adr.md) records choices for adoption.

Numbering has been reorganized. A reference to an old section SHALL retain its
old source revision; `RC.1 §33.8` is not silently redirected to RC.3 §8. Requirement
IDs remain in §106 with explicit scope and changed meanings. The RC.1 document
is the source for legacy contracts, not a second set of RC.3 requirements.

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
| Verification | Independent evaluation of an exact result against bounded obligations; optional for ordinary work and required for common qualification |
| Completion | The requested Warrant work is finished and recorded as complete; qualification and human signing are separate |
| SDK | OpenWarrant library of document/record types, codecs, validators and consumer interfaces |
| Semantic compiler | External implementation that resolves source dependencies and assembles context under a declared profile |
| Agent act | Authorization or completion attributed to an agent; an authority claim requires its policy basis, while unverified completion needs no prior OpenWarrant authorization; not human acceptance |
| Verified mark | Display of the common assurance qualification, requiring evidence and secure human acceptance |
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
is an edition-specific act, not an alias for RC.3 acceptance or qualification.
`war`, frontier, battery, plant, drafter and proposal are reference-tool vocabulary;
their existence does not add requirements to basic document compatibility.

<!-- ow:unit documents binding -->
## 4. Documents, identities, and authoring

The normal editable source SHALL be one Markdown file with a TOML metadata
footer and defined units. Human-readable content comes first; compact dependency
arrays and exact metadata framing follow the RC.3 format contract. Historical
RC.2 header sources retain their explicit adapter and original bytes. The format contract fixes required fields, delimiters,
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

A Warrant or its explicitly referenced SAS MAY define a gate that requires
verified preparation, a verified prerequisite or signoff before starting or before
another named action. Conditions SHALL distinguish qualification-only requirements
from enforced action gates, naming their scope and timing. An action gate cannot
be bypassed by relabeling the attempt unverified. Changes to it follow the declared
amendment process. There is no global requirement that every Warrant have one.

Qualification-only conditions remain visible without forcing a signing step before
ordinary work. Joint project approval, independent stage readiness and mixed
arrangements are contract choices. Verified preparation concerns declared pre-work
facts; it does not establish correctness of an implementation that does not exist.

A selected professional workflow MAY check its declared pre-work requirements
before starting. Those include outcome/scope, exact basis, outputs/checks, approvals,
dependencies, limits and stopping conditions. Such a workflow must be explicitly
selected; it is not required merely because work uses a Warrant. Actual resource
access and harness limits remain separate from OpenWarrant qualification. Practical
dependencies can prevent execution regardless of whether verification is requested.

A requested material change to outcome, architecture, authority, scope, or
mandatory expectations SHALL produce a reviewable revision. A policy may delegate
such approval as §11 allows. Earlier attempts and their evidence remain bound to
the old basis. A refreshed packet is not an implicit amendment.

<!-- ow:unit compilation binding -->
## 6. SDK architecture and external compiler boundary

The SDK SHALL expose narrow, typed public operations for parsing, validating,
authoring, reading source units, canonical encoding/digests, checking supplied
records, and exchanging provider requests/results. Proposed API responsibilities,
error behavior and runnable cases are in [SDK contract](sdk-contract.md).
The implementation retains a Rust library boundary; CLI and language adapters
SHALL call shared semantics rather than duplicate validators. CLI is an I/O shell,
not the only route to a primitive. Phase 1 implements each scoped primitive with
direct examples and tests, then exposes that functionality through CLI commands.

The SDK SHALL retain original bytes separately from parsed values. It validates
pointer/condition syntax and preserves dependencies without resolving remote or
cross-document meaning. Semantic compilers use those types to implement:

**Capture/resolve → assemble → select → render/package → check full closure.**

The SDK may check manifest structure, hashes and declared reference ranges against
supplied bytes. It SHALL report that bound separately from a provider's recomputed
dependency-coverage proof. Complete semantic validation belongs to the qualified
compiler interface; a schema or digest check cannot silently claim it.

A conforming deterministic compiler path SHALL consume explicit captured inputs
and produce reproducible context without a model. Retrieval, summary drafting and
source acquisition are optional upstream steps with their own provenance. SDK calls
SHALL NOT hide network/model work. Provider identity, profile version, diagnostics,
unsupported capabilities, completeness and readiness remain explicit at the adapter.
LAMU owns its compiler implementation and optional memory/retrieval backend; other
providers may implement the same profile. This SAS does not select LAMU's database.

Canonical structured JSON SHALL use RFC 8785; raw digests hash original bytes.
Digest domains and signed subjects remain versioned. IDs, timestamps, host paths
and runtime facts enter reproducible inputs only when explicitly supplied. Earlier
canonical objects and signatures SHALL NOT be reinterpreted by a new codec.

<!-- ow:unit selection binding -->
## 7. Context selection and dependency coverage

The compiler SHALL select explicit references and evaluate structured conditions
without AI. The retained context-profile conditions cover stage, subsystem, and repository-relative file
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

Readers claiming full compiler-profile conformance SHALL check package digests,
recomputed required membership, schema, source spans and content references.
SDK integrity-only inspection SHALL label semantic coverage as not evaluated (§6).
A digest establishes consistency
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
## 11. Permission, agent acts and human acceptance

Resource access, OpenWarrant authorization, implementation completion, verification
findings, human acceptance, qualification, governance adoption, merge and deployment
are distinct. Any human or agent MAY execute any Warrant on the unverified path;
the standard SHALL NOT impose a universal prior authorization record, start-condition
pass, policy-enrollment ceremony or signature. Explicit Warrant action gates still
apply, and unverified mode cannot waive them. The reference prompt-only
workflow SHALL act on the user's instruction and report completion distinctly as
unverified. Optional verification remains available before release or at review.

Recorded acts bind their subject, actor, meaning and available evidence. A policy
basis is required when claiming authority under that policy, not for merely
reporting unverified work. An agent MAY sign an attributable completion record,
but a signature is optional for unverified execution and completion reporting.
It SHALL NOT sign as a human or turn completion into human acceptance or a mark.
Parsing a record or setting a state field does not authenticate it. A prompt is
an instruction to work; it does not pretend to be a cryptographic signature.

The standard does not grant repository credentials, bypass sandbox controls or
increase user-set spend limits. These execution capabilities are separate from
whether the work satisfies OpenWarrant qualification requirements. Missing formal
OpenWarrant authorization alone SHALL NOT block an unverified path whose contract
does not require it as an action gate.

Named human-approved policies MAY delegate architecture amendments or SAS adoption;
direct human approval remains their default. Delegated adoption is labeled as such
and never satisfies the common mark's human acceptance requirement. Only an
authorized human SHALL approve changes to effective automation permissions,
including indirect changes through SAS, parent policy or adapter configuration.
Automation cannot expand its own effective authority.

The selected workflow, assurance standing, applicable policy, scope and limits
SHALL be visible. Professional workflows may require pre-work review and manual
handling. A prototype Warrant may remain complete and unverified indefinitely,
with or without an agent signature, without creating a mandatory acceptance task.
Qualification may be requested later. A stricter deployment/merge workflow may
require it. The word Verified used as a product mark SHALL mean §12 qualification;
test execution and independent findings receive their own explicit status labels.

<!-- ow:unit assurance binding -->
## 12. Optional assurance baseline

The provisional profile identifier is `oh.war/assurance-baseline/1.0.0-rc.3`.
This successor strengthens acceptance authentication; RC.2 qualifications retain
their original profile and do not automatically qualify under this one.
The public mark name remains a branding choice, not a build dependency. Repositories
MAY strengthen this profile but SHALL NOT weaken it while claiming the same mark.
Qualification covers one Warrant result, exact code revision, contract, and scope.

All baseline conditions SHALL be satisfied:

1. The result has a bounded outcome and scope, exact source/contract basis, and
   truthful work history. The authorization and starting requirements applicable
   to its qualification profile are satisfied with their actual timing. Prior
   OpenWarrant authorization is not a universal requirement of this final-result
   baseline. The release review can authorize acceptance of the completed result;
   it cannot claim an earlier authorization that did not occur. Explicit temporal
   requirements in a stronger profile still require evidence for that time.
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
   line-by-line human review unless that actually occurred. Acceptance SHALL be
   authenticated by an attributable human-controlled SSH signature or another
   explicitly supported secure signing method binding the exact documentation,
   contract, result and verification evidence. A logged-in session alone, a
   human-kind field, a shared service key or an agent using the human's key cannot
   satisfy this requirement. The trust adapter must establish the human act.
   Policy SHALL NOT waive the human-signature requirement for the common mark.
8. The qualification record binds the profile revision, Warrant/contract, code and
   artifact digests, checks, independent verification, human acceptance, scope,
   limitations, and recoverable supporting evidence. Its authenticity is checkable
   under the repository's declared trust policy.

Later qualification is allowed. Fixtures added after prototyping may satisfy this
final-result baseline when protected for verification; they SHALL NOT establish
that tests existed before implementation. A stricter `fixtures-before-work`
profile requires preparation and joint approval of fixtures and Warrant before
execution. The same timing rule applies to explicit approvals-before-start and
other pre-work conditions. An unmet temporal requirement blocks that qualification,
not unverified work. Use a qualifying new attempt where needed; never backdate a
condition, silently weaken its profile or claim historic process compliance.

A release review MAY combine multiple completed Warrants into one human review
and secure signing ceremony. Its exact manifest SHALL bind every included Warrant
revision, result, evidence set and selected profile. Each result independently
meets all applicable conditions; one passing member does not qualify another.
A batch acceptance must explicitly cover those exact members. This reduces
signing effort without replacing detailed agent/human review or granting a mark
to an entire repository through one selected change.

Changing the accepted candidate, required expectations, or relevant verification
basis requires new verification and acceptance for the new result. The earlier
mark remains historical at its old revision. Dispute, invalidated evidence, or
annulment SHALL be recorded as standing, without deleting the earlier fact.
One marked Warrant SHALL NOT qualify a whole repository or release.

<!-- ow:unit records binding -->
## 13. Records, history, and successor work

The SDK SHALL inspect supplied record facts through the small record
interface in the format contract. It SHALL preserve claim, observation, inference,
judgment, verification, acceptance, and qualification as distinct classes.
It does not mint authoritative evidence by parsing an agent-authored report.
Record-only validation reports its trust and runtime limitations explicitly.

Work state SHALL separate lifecycle phase, execution condition, result,
currency, and acceptance/qualification standing. Unverified and verified work use
the same completion state: complete. An agent or human MAY record completion of
the requested work without a human signature or satisfaction of qualification-only
requirements. The ordinary path SHALL NOT leave that Warrant in a lesser pending
completion state merely because it lacks verification. `Complete, unverified` and
`complete, verified` distinguish assurance, not two levels of work completion.

Progress trackers SHALL count both as completed work and report qualification
coverage separately. Independent findings retain their actual verdict; the Verified
mark requires §12 including secure human acceptance. A program expressly seeking
qualification can still have that separate goal pending. Completion cannot falsely
claim unfinished requested implementation is done. These are new-edition semantic
distinctions, not values silently added to the legacy state enum. Superseded, disputed,
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

The ungated prompt-only path SHALL be prompt → execute/questions → record the Warrant as
complete and unverified → return the completion response. Draft or reuse the
Warrant without a mandatory signing interruption. The user's next prompt can
start the next Warrant without a review or signature ceremony for the completed one,
unless an explicit prerequisite or action gate requires it.
Verification can wait until release: gather results → prepare/protect checks →
independent verification → human review and secure batch acceptance → qualification
of each eligible exact result. Release review supplies real scrutiny; it is not a
signature-only conversion from unchecked work to Verified.

The optional professional path is prepare → review authorization/start conditions
→ execute/questions → independent verification → human acceptance. A separate
preparation agent reuses or drafts fixtures; the human reviews them with the
Warrant where the selected profile requires that act. Pre-work review establishes
plan/readiness, not correctness of code that does not yet exist. Ordinary start
approval may use an authenticated, unlocked app session; secure human acceptance
for the common mark remains mandatory (§12). Merely seeking later final-result
qualification does not require the professional pre-work path.

The reference workflow app and its shared API SHALL support a connected agent using an
OpenWarrant skill and a configured coding-agent command launched by the service.
If no eligible agent is available, show waiting-for-agent and start when one is
available. A claim is not execution; execution is not acceptance.

Each Warrant SHALL have one isolated Git worktree per participating repository,
with writers serialized within each worktree. Independent Warrants may run concurrently. A verifier uses separate context
and workspace. The harness supplies sandbox enforcement; the workflow checks the
required protections and records the evidence. Neither a skill nor a worktree alone is a sandbox.

Every executing agent SHALL have a hotline. In-scope technical questions go to an
AI adviser first; governing questions go to an authorized decision-maker. Agents
may request direct human escalation. Advice is not a grant of authority. Store
question, exact basis, respondent, answer/evidence, and affected stages. Pending
blocking answers pause affected work and dependents, while independent work
continues. No eligible responder means visible waiting, not guessed permission.

### 14.1 Work-stop response and generated overview

Every work stop SHALL produce a standardized, configurable response, whether it
finishes a feature, integration unit, Warrant or the declared program. The response
is the same kind for unverified and verified work; its scope and assurance standing
remain distinct. Harness/agent interruption is not completion of a work unit.

The normal response begins with the configured safeword, exactly as configured on
its own first line, and links to a deterministically generated progress overview.
The control-position safeword is emitted only after the declared work unit completes.
The same word may appear in ordinary conversation without acting as a completion
event. Meaning comes from the structured work-stop record and scope, not a keyword
search. Completing a feature must not claim that its whole Warrant is complete.

The linked overview SHALL identify its tracker source, revision/as-of basis and
completion event; show pending work and completed work; separate qualification
standing; and provide configurable progress bars or circles, statistics, metrics
and other data views. Unknown values remain unknown. Visualizations are computed
from records with a stated scope and denominator, not invented by an agent.

Implementation notes, generated document trail and next steps SHALL be available
in the response or its linked documents. The trail names actual artifacts and their
source/version links. Agent-authored interpretation is distinguishable from
generated tracker facts. Next steps distinguish recorded remaining work from
suggestions. Default chat output uses concise pointers rather than pasting the
overview or large documents. Configuration can reduce inline fields, note length
and output tokens, or select richer views, without losing the required information
from the linked package or hiding actual state and qualification.

The tracker and deterministic renderer/compiler produce project-state documents;
the agent obtains their links and supplies bounded implementation notes. Rebuilding
the same view from the same source basis/configuration requires no model call and
produces the same content. Stable navigation may point to the latest overview;
the work-stop record retains the exact referenced snapshot for historical review.
Pointers must resolve for the intended reader and must not silently identify an
older view that lacks the completed event. A newer compatible view may include
concurrent work while preserving the named result and its history.

If work finishes but tracker synchronization or overview generation fails, report
completed work and the actual sync/delivery problem. Preserve the completion for
retry; do not invent a document link, revert completed work or emit a normal
confirmed-response signal. Retry uses the same event and does not rerun the work.

OpenWarrant defines the response/record contract and SDK helpers. The tracker,
view provider and workflow own storage, rendering and dispatch. A subsequent prompt
starts the next requested work, subject to any explicit Warrant action gates; a
work-stop response or next-step suggestion is not itself a new execution command.

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
standard imposes no dollar limit on every consumer. The bundled workflow's default
spend cap is USD 10 per run. A human may configure another amount or explicitly
leave the cap undefined. An enabled hard cap requires reliable cost accounting
and enforcement before paid calls; do not spend first and discover the overrun
afterward. Unknown cost is allowed only when policy permits and no mandatory hard
cap needs that accounting, and is shown as unknown, never zero. These are workflow
defaults, not requirements for document validity or free SDK operations.

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
## 16. Integrations, skills and developer friction

Phase 2 SHALL qualify SDK consumption by LAMU and a separately implemented minimal
consumer, with pinned protocol/build/fixture identities. LAMU is the first-party
semantic compiler; it is not an SDK runtime dependency. Provider-specific behavior
stays behind typed adapters. Unsupported capabilities, unavailable providers,
incomplete context and stale source bases produce explicit refusals, not fallback
to an apparently conforming result.

Phase 3 SHALL exercise a real reference webapp built on the SDK. Its integration
matrix covers the first-party applications named in [phase plan](phase-plan.md),
test applications, fixtures and actual users. Workflow ownership remains in those
apps. Knowledge Fabric owns its institutional record/authority boundary; Katana
agent execution; BLUT computational orchestration; Liminal its source semantics.
External trackers may consume adapters without redefining document semantics.

OpenWarrant SHALL maintain its own adapted workflow skill suite based on Matt
Pocock's skill library, with upstream attribution, revision provenance and retained
license notices. OpenWarrant owns its adaptations, not upstream authorship. The
[skill adaptation contract](skill-adaptation.md) defines invocation, outputs,
required context pointers and evaluation. Invoking a war skill SHALL produce a
typed OpenWarrant artifact appropriate to the job: Warrant draft, linked ADR,
decision/question record, stage plan, or bounded review evidence. Optional HTML,
tracker entries and chat summaries are views or links, not substitute authorities.
The SDK validates these artifacts; a skill does not reimplement the standard.

OpenWarrant integrations SHALL preserve existing host context documents, including
`AGENTS.md`, `CLAUDE.md` and `CONTEXT.md`, with their native ownership and scope.
Use conditional pointers to shared guidance rather than replacing host instructions
or requiring their conversion to OpenWarrant format. The harness owns instruction
precedence. Providers retain exact source identity and applicable rules; the SDK
validates supplied references. The [integration contract](skill-adaptation.md#integration-with-existing-context-documents)
defines additive edits, source delivery and conformance cases. Filenames and
retrieval results do not confer authority.

Skills SHALL obey §11's effective permission mode, record real actor identities,
and distinguish proposed, executed and independently verified acts. They cannot
grant themselves policy or the human-backed mark. User decisions already supplied
in the session SHALL be reused; routine context gathering and reversible drafting
do not introduce redundant approval rounds.

Ordinary setup targets 5–10 minutes or less; advanced discussion remains optional.
After setup, routine workflow administration SHALL avoid manual metadata editing
and mandatory user shell commands, targeting at most 60 seconds per Warrant.
Measure administration, substantive review, waiting and total time separately.
The reference workflow proves a meaningful OpenWarrant maintenance change without
requiring unrelated historical Warrants to close. These are targets, not measured
results from this amendment.

Where a selected workflow requires a human act, its configured interface SHALL
present the exact subject and accept a concise action: a checkbox/button or a
single CLI command such as `war sign <designation>` in a supporting CLI. Preparation,
metadata and requests are generated. A short gesture does not weaken identity,
exact-subject binding or §12's secure human acceptance. A human should not need a
second terminal or a paragraph of repeated metadata when the harness can provide
the configured approval interface. Stronger policy may require extra confirmation.

<!-- ow:unit phases binding -->
## 17. Four phases and release criteria

1. **Library and Standard:** finalize document/record semantics and SDK scope;
   implement every scoped primitive with directly runnable files/examples and
   admitted/refused/boundary fixtures; expose those primitives through the CLI.
2. **SDK and Compiler Integration:** qualify SDK use by LAMU and other low-level
   consumers; test exact preservation, semantic compiler-profile interoperability,
   capabilities, refusals, offline exchange and compatibility. OpenWarrant does
   not implement the provider's semantic compiler.
3. **Workflow Integration:** deliver a real SDK-based reference webapp, first-party
   app integrations, reference/test consumers, complete workflow fixtures and
   actual-user evidence for prototype and human-qualified paths.
4. **Hardening and Standard Adoption:** security, reliability, packaging,
   compatibility, independent conformance, documentation, generalization and
   public adoption. Publish Stable only through an explicit owner release act
   after required evidence; writing a standard or completing Phase 2 is not release.

[Phase plan](phase-plan.md) owns gates and integration coverage. [Phase 1 scope](phase-1-build-scope.md) owns the bounded primitive inventory and direct checks.
Each claimed capability SHALL have observed named cases and refusal controls.
Neither a passing sample nor an aggregate count proves every application or
absence of all failures. Linux/macOS SDK and CLI compatibility remain required.

The old eleven-feature/compiler roadmap is reassigned in [migration map](migration-map.md). External compiler conformance keeps exact rules, conservative
unknown applicability, full required dependencies and portable context. Ownership
changes SHALL NOT weaken those semantics. Context optimization claims require
repeatable task evaluation including omissions, wrong actions, bytes/tokens and
human effort; SDK publication need not claim a model-efficiency improvement.

Generalization supports minimal prototypes, documented production, and stronger
licensed/security/regulatory profiles. Such profiles must name their rules and
evidence; the common mark SHALL NOT claim legal compliance, license clearance or
universal security merely because a human signed. Public promotion and any such
assurance claims remain bounded by their demonstrated profile and exact scope.

<!-- ow:unit adoption binding -->
## 18. Compatibility, adoption and governance

This amendment changes ownership, release phases and permitted actor semantics.
It does not edit RC.2's retained source bytes, historical `1.0.0` acceptance,
authorized Warrant atoms, attestations or earlier evidence. Owner acceptance in
conversation is design direction; this draft does not fabricate signed adoption.
Adoption SHALL bind this complete source set and its architecture decision.

The RC.2 document grammar, reference/packet profile and example bytes remain
explicitly versioned compatibility inputs in this draft. Their `1.0.0-rc.2` schema
IDs denote wire versions, not acceptance of SAS RC.2. The SDK SHALL dispatch by
schema, never infer a new dialect from Markdown or replace signed bytes in place.
RC.3's agent-act contract is separate from those existing record envelopes; the
SDK contract defines its proposed wire envelope and trust boundary. Old consumers
must report unsupported semantics rather than reinterpret an agent act as human
acceptance. RC.3 assurance uses a new baseline identifier (§12).

Previously numbered requirements retain IDs with explicit successor meanings.
Old references resolve against their original editions. Existing signed Warrants
remain historical facts. Unsigned implementation plans may be revised against
this new basis; signatures and resolutions are separate acts. [Migration map](migration-map.md) assigns every planned OW-WAR-0074–0091 and RC.2 F/T case without
claiming cancellation, supersession, implementation or a new allocated identifier.

Repository AGENTS.md may route tasks to current project guidance, native domain
documents and this candidate while retaining a linked legacy workflow reference.
Existing legacy templates, installed war skills and CLI enforcement do not become
prototype-capable by this document edit. Prepare scoped successor work for their
policy/SDK migration, retaining protected history and explaining source-edition
differences. Legacy command meanings remain
until versioned migration; a new SDK wrapper must not silently alias `war compile`
to another compiler. Root policy changes are not implicit in this draft.

Legacy import preserves original states, identities, source holders, signature
subjects and immutable bytes. A resolved state alone cannot mint new acceptance
or qualification. A successor changes the live delivered version with explicit
lineage, without generating correction fanout to every earlier Warrant. Actual
correction of an old assertion remains separately attributable.

<!-- ow:unit context-views binding -->
## 19. Context views, shared work and retention

[Context views and shared work](context-views-and-shared-work.md) is normative.
It defines the supervisor progress view, decisions/domain view, draft scratchpad,
code-derived contract view, skill pointers and context exclusion state as views of
one owned source model. Their display names are provisional; their distinctions
are binding. SQL and vector retrieval belong to context providers. A task retains
its Warrant, role, scope, inputs, checks and limits independently of its query.

Cross-project work shares one exact Warrant/adapter contract rather than divergent
common requirements. Participant permissions and local completion remain scoped;
the shared outcome requires its own declared integration evidence. Start gates
and qualification-only conditions keep the meanings in §5.

Context suppression, quarantine and permitted evidence destruction remain distinct.
Active required rules use §20 before exclusion. Evidence deletion preserves signed
history and records which assurance can no longer be substantiated. Referenced
decisions and prior accepted revisions remain retained under §10.

<!-- ow:unit stops binding -->
## 20. Stops, failures and applying changes

[Work-stop contract](work-stop-contract.md) is normative. A work stop records a
declared unit's completion; an agent stop records interruption or suspension with
possibly unfinished work. Feature, integration, Warrant and program identify scope.
Tool boundaries admit harness updates and do not themselves imply a work stop.
Work state, cause, worker execution, review and assurance remain separate facts.

For effective SAS, architecture, Warrant or other work changes, the user chooses
stop now or continue under the recorded old basis until the next named work stop.
Required rules remain in context until affected writers stop. Preserve old state,
apply the permitted change/archive transition and deliver the new required context
before resuming. A harness-only update applies before the next affected tool action;
it cannot silently change work requirements or expand authority. Mandatory limits,
revocation and explicit cancellation take precedence over deferred continuation.

Every work stop uses §14.1's configured safeword and pointer response. Agent stops
report saved state, unfinished work, cause, live operations and resume conditions
when possible. Missing reports, unknown live writers and retry uncertainty remain
explicit; a turn ending or tool timeout never proves a background writer stopped.

<!-- ow:unit requirements binding -->
## 106. Normative requirement index

The index preserves existing IDs for traceability. Titles below describe their
RC.3 meaning, not a claim that historical contracts used that meaning. `D` means
document/SDK, `C` external compiler profile, `A` assurance records, `W` workflow,
and `L` legacy compatibility. Build coverage names the new owner/phase; the migration
map retains exact RC.2 F/T traceability. The decision map records major
supersessions. Source clauses, not titles alone, define full requirements.

| ID | Requirement | Scope | Source section | Build coverage |
| --- | --- | --- | --- | --- |
| WAR-SAS-RQ-001 | Document identity is stable; legacy UUIDs are preserved and new shared identities should use UUIDv7 | D/L | 4, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-002 | Local aliases do not substitute for source identity or authority | D/L | 4, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-003 | Selected federation authorities allocate their official enterprise identity | D/L | 4, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-004 | Registration and import do not silently transfer source ownership | D/L | 4, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-005 | Federated cross-repository relations retain their issuing and local authorities | D/L | 4, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-010 | New documents and legacy authored atoms remain editable sources before immutable approval | D | 4, 6 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-011 | Bound source content changes through its owning authority | D | 4, 6 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-012 | Generated views are regenerated from sources rather than edited as authorities | D | 4, 6 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-013 | Document composition is typed, ordered, and deterministic | D | 4, 6 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-014 | Human and canonical machine views derive from one captured source basis | D | 4, 6 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-015 | Missing required source content prevents a complete task packet | D | 4, 6 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-020 | Material governing architecture decisions remain first-class ADRs; in-scope advice is not a new decision | D/W | 4–5, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-021 | Decision indexes are generated from source records | D/W | 4–5, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-022 | Work traces to applicable SAS requirements; no SAS or roadmap is required for every valid prototype draft | D/W | 4–5, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-023 | Child Warrants cite their exact parent basis | D/W | 4–5, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-024 | Children do not rewrite parent rationale | D/W | 4–5, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-025 | Successor work preserves earlier Warrants and their historical standing | D/W | 4–5, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-030 | Approved contract snapshots are immutable | D/A/W | 5, 11, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-031 | Progress cannot amend a contract | D/A/W | 5, 11, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-032 | State separates lifecycle, execution condition, result, currency, and acceptance or qualification standing | D/A/W | 5, 11, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-033 | Material amendment creates a new contract revision | D/A/W | 5, 11, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-034 | Prior attempts retain their original basis | D/A/W | 5, 11, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-035 | Workflow readiness checks its required inputs; document validity is separate | D/A/W | 5, 11, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-036 | Successor work preserves old delivered bytes and evidence without freezing a live pathname; corrections remain separate | D/A/W | 5, 11, 13 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-040 | Milestones and execution stages are distinct | W | 5, 9, 14–15 | Workflow Phase 3 |
| WAR-SAS-RQ-041 | Typed stage ports remain a selected orchestration adapter contract | W | 5, 9, 14–15 | Workflow Phase 3 |
| WAR-SAS-RQ-042 | An executing actor receives an exact task dispatch basis | W | 5, 9, 14–15 | Workflow Phase 3 |
| WAR-SAS-RQ-043 | Task context exposes basis, required outputs, constraints, limits, and stopping conditions | D/W | 5, 9, 14–15 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-044 | Execution authority is explicit and bounded by human-approved effective policy | D/W | 5, 9, 14–15 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-045 | Replay, repair, and worker recovery retain distinct attempt history and limits | W | 5, 9, 14–15 | Workflow Phase 3 |
| WAR-SAS-RQ-046 | Entry token estimates include rendering overhead; exceeding a required-content budget is a refusal | D/C | 5, 9, 14–15 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-050 | Assurance qualification decomposes completion claims into bounded obligations | A | 12–13 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-051 | Claims declare scope bounded by their evidence | A | 12–13 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-052 | Claim, evidence, observation, inference, judgment, acceptance, and qualification remain distinct | A | 12–13 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-053 | Performer reports cannot satisfy independent verification | A | 12–13 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-054 | Required unknown or failed checks block assurance qualification | A | 12–13 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-055 | Assurance requires adequacy evidence with applicable refusal and control cases | A | 12–13 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-056 | Required check definitions and expectations are versioned and protected from performer changes during verification | A | 12–13 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-057 | Invalidated evidence changes dependent qualification standing without rewriting history | A | 12–13 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-058 | Residual risk requires explicit authorized human acceptance and cannot waive a failed baseline requirement | A | 12–13 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-059 | Human acceptance binds exact contract, result, and assurance evidence | A | 12–13 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-060 | Knowledge Fabric owns authority and lifecycle for its selected federated workflow, not every standalone compiler use | W/L | 2, 16, 18 | Workflow Phase 3 |
| WAR-SAS-RQ-061 | OpenWarrant owns the standard and SDK; LAMU and other providers own semantic compilation | D | 2, 16, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-062 | Selected Katana adapters retain Katana runtime and PromptIR ownership | W/L | 2, 16, 18 | Workflow Phase 3 |
| WAR-SAS-RQ-063 | Selected BLUT adapters retain BLUT typed execution ownership | W/L | 2, 16, 18 | Workflow Phase 3 |
| WAR-SAS-RQ-064 | The SDK runs without sibling runtimes; OpenWarrant does not duplicate a production semantic compiler | D | 2, 16, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-065 | Native systems retain their artifact and source authority | W/L | 2, 16, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-070 | SDK authoring and validation work offline; the conforming compiler path remains model-free | D | 4, 6, 16–17 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-071 | Agent-assisted planning returns reviewable structured source proposals | W | 4, 6, 16–17 | Workflow Phase 3 |
| WAR-SAS-RQ-072 | Proposals are validated before source writes and cannot grant themselves authority | D | 4, 6, 16–17 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-073 | Planning identifies material architecture decisions for ADR proposals | W | 4, 6, 16–17 | Workflow Phase 3 |
| WAR-SAS-RQ-074 | Document and compiler checks are deterministic and agent-free | D | 4, 6, 16–17 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-075 | Generated views are checked against their exact source basis | D | 4, 6, 16–17 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-076 | Federated controlled actions use the owning authority interface rather than direct status edits | W | 4, 6, 16–17 | Workflow Phase 3 |
| WAR-SAS-RQ-077 | Selected code, document, and run workflows each require their own observed gates and examples | W | 4, 6, 16–17 | Workflow Phase 3 |
| WAR-SAS-RQ-080 | Canonical structured preservation uses RFC 8785 JSON | D/L | 6, 10, 13, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-081 | Digests use explicit algorithms and versioned domains; original source bytes remain recoverable | D/L | 6, 10, 13, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-082 | Preservation exports retain contract, sources, receipts, assurance and acceptance when present without inventing missing history | D/L | 6, 10, 13, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-083 | Export-import-export preserves semantic identity and original evidence bytes | D/L | 6, 10, 13, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-084 | Historical superseded, disputed and annulled records remain available | D/L | 6, 10, 13, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-085 | Existing SSH-signed acts retain their portable attestations and original subjects | D/L | 6, 10, 13, 18 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-090 | Master context is a generated assembly of exact source revisions; source authority remains at its origin | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-091 | Binding units retain exact text with required definitions, conditions, exceptions and schemas | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-092 | Context pointers have stable targets, explicit references and versioned structured conditions | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-093 | Unknown applicability includes an available rule and its dependencies while preserving uncertainty and authority blockers | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-094 | Required dependency coverage is explicit, terminating and provenance-preserving; inconsistent or unresolved required meaning cannot be complete | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-095 | Background summaries preserve source provenance, trust, classification and taint and never replace binding text | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-096 | Complete explicit inputs compile reproducibly without a model call or mandatory sibling runtime | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-097 | Agent packets contain a task brief, binding context, reference catalog and integrity-bound source manifest | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-098 | V1 exports all required task context for offline reading without the original repository or running service | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-099 | Optional resolvers and absent optional references remain distinct from required bytes actually supplied | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-100 | Role-specific packets preserve common requirements, independence and access to full required evidence | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-101 | Budget accounting covers compiler rendering overhead and distinguishes estimates from observed harness usage | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-102 | Required context is never discarded to satisfy a budget | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-103 | Source, scope, selection, policy and representation changes invalidate affected projections without rewriting earlier packets | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-104 | Stable source-unit identities survive heading changes and retain exact source locators | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-105 | Task instructions expose bounded outputs and observable completion criteria with supporting material progressively disclosed | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-106 | Generators derive indexes and digests without requiring manual duplicate metadata maintenance | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-107 | Context optimization claims are bounded by repeated task evaluation including omissions, wrong actions, tokens and human effort | D/C | 6–10, 17 | SDK S01/S03/S05; external compiler Phase 2 |
| WAR-SAS-RQ-108 | Four-phase qualification precedes the explicit Stable release and public adoption claims | D | 6–10, 17 | Phases 1–4 |
| WAR-SAS-RQ-110 | Minimal Markdown documents are valid independently of workflow readiness | D | 2, 4 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-111 | Structured source grammar and extension interpretation are versioned and reject unsupported required semantics | D | 4; F1–F4 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-112 | Unverified work needs no universal approval; explicit Warrant start/signoff gates remain enforceable and distinct from qualification-only conditions | A/W | 5, 11 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-113 | Only an authorized human approves changes to effective automation policy, including indirect changes | A/W | 11 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-114 | Common assurance requires independent evidence and secure human-signed acceptance of the exact result | A | 12 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-115 | Later qualification preserves actual fixture timing and does not claim fixtures-before-work retrospectively | A | 12 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-116 | Every work stop returns configured safeword and concise pointers to deterministic progress, notes, document trail and next steps; qualification remains separate | W | 11–14 | SDK S04/S05; workflow Phase 3; hardening Phase 4 |
| WAR-SAS-RQ-117 | Default main-branch merge requires assurance; human policy may permit unmarked merges; deployment is separate | W | 15 | Workflow Phase 3 |
| WAR-SAS-RQ-118 | Writers serialize per Warrant worktree and replacement requires proof prior writer cannot write | W | 14–15 | Workflow Phase 3 |
| WAR-SAS-RQ-119 | Blocking hotline questions pause affected work and dependents while independent work continues | W | 14 | Workflow Phase 3 |
| WAR-SAS-RQ-120 | Repair count is user-selectable with three cycles as fallback; unknown cost never implies an enforced cap | W | 15 | Workflow Phase 3 |
| WAR-SAS-RQ-121 | Phase 1 proves each SDK primitive directly and then through CLI access | D | 17 | SDK Phase 1; integration Phase 2; see migration map |
| WAR-SAS-RQ-122 | SDK exposes typed document, record and adapter primitives independently of compiler/service execution | D | 2, 6 | S01–S07 |
| WAR-SAS-RQ-123 | First-party war skills adapt Pocock methods into validated OpenWarrant artifacts with attribution and version provenance | D/W | 16 | S08; Phase 3 |
| WAR-SAS-RQ-124 | Skills reuse settled decisions, integrate scoped host context through explicit pointers and report observable completion without manufacturing authority | D/W | 16 | S08; Phase 3 |
| WAR-SAS-RQ-125 | SDK/compiler integration validates provider capabilities, exact subjects, explicit refusals and claimed coverage | D/C | 6, 16 | Phase 2 |
| WAR-SAS-RQ-126 | Reference webapp proves prototype and human-qualified paths with first-party integrations and actual users | W | 14–17 | Phase 3 |
| WAR-SAS-RQ-127 | Agent signatures never satisfy the mandatory secure human acceptance for the common mark | A/W | 11–13 | S04/S05; Phase 3 |
| WAR-SAS-RQ-128 | Public standard and stronger-profile claims are bounded by observed hardening and conformance evidence | D/A/W | 17 | Phase 4 |
| WAR-SAS-RQ-129 | Context views preserve source ownership, separate progress/maturity/assurance and distinguish intended APIs from code-derived facts | D/C/W | 19 | SDK-19; CTX-01/03; Phases 2–3 |
| WAR-SAS-RQ-130 | Query-driven context uses exact authorized revisions and required dependency edges; SQL or similarity never grants authority | D/C | 7–9, 19 | SDK-20; CTX-02/04/08; Phase 2 |
| WAR-SAS-RQ-131 | Cross-project work uses one shared contract revision with scoped participants, start conditions and integration evidence | D/C/W | 5, 19 | SDK-21; CTX-05; Phases 2–3 |
| WAR-SAS-RQ-132 | Context exclusion and evidence destruction preserve signed history and disclose affected assurance; active rules follow stop/change handling | D/C/A/W | 10, 19–20 | SDK-22; CTX-06/07; Phases 2–3 |
| WAR-SAS-RQ-133 | Work and agent stops separate completion, cause, actual execution and assurance at an explicit scope | D/W | 14.1, 20 | SDK-17/23; stop scenarios; Phase 3 |
| WAR-SAS-RQ-134 | Work changes honor recorded user choices; harness updates apply before affected dispatch with fencing and recovery evidence | D/W | 15, 20 | SDK-24; stop scenarios; Phases 3–4 |

<!-- ow:metadata -->
<details>
<summary>OpenWarrant metadata</summary>

```toml
schema = "oh.war/document/1.0.0-rc.3"
kind = "sas"
id = "openwarrant:sas"
revision = 3
title = "OpenWarrant Software Architecture Specification"
state = "proposed"
dependencies = [
  { unit = "documents", target = "format-contract.md#source" },
  { unit = "documents", target = "format-contract.md#units" },
  { unit = "compilation", target = "format-contract.md#request" },
  { unit = "compilation", target = "format-contract.md#digests" },
  { unit = "selection", target = "format-contract.md#pointers" },
  { unit = "selection", target = "format-contract.md#conditions" },
  { unit = "selection", target = "format-contract.md#selection" },
  { unit = "packets", target = "format-contract.md#package" },
  { unit = "records", target = "format-contract.md#records" },
  { unit = "assurance", target = "format-contract.md#records" },
  { unit = "phases", target = "phase-1-build-scope.md" },
  { unit = "documents", target = "#context" },
  { unit = "contracts", target = "#context" },
  { unit = "compilation", target = "#context" },
  { unit = "selection", target = "#context" },
  { unit = "provenance", target = "#context" },
  { unit = "packets", target = "#context" },
  { unit = "invalidation", target = "#context" },
  { unit = "authority", target = "#context" },
  { unit = "assurance", target = "#context" },
  { unit = "records", target = "#context" },
  { unit = "workflow", target = "#context" },
  { unit = "recovery", target = "#context" },
  { unit = "integrations", target = "#context" },
  { unit = "phases", target = "#context" },
  { unit = "adoption", target = "#context" },
  { unit = "contracts", target = "#authority" },
  { unit = "assurance", target = "#authority" },
  { unit = "assurance", target = "#contracts" },
  { unit = "assurance", target = "#records" },
  { unit = "records", target = "#assurance" },
  { unit = "packets", target = "#invalidation" },
  { unit = "compilation", target = "sdk-contract.md" },
  { unit = "integrations", target = "skill-adaptation.md" },
  { unit = "phases", target = "phase-plan.md" },
  { unit = "context-views", target = "context-views-and-shared-work.md" },
  { unit = "context-views", target = "#context" },
  { unit = "context-views", target = "#authority" },
  { unit = "stops", target = "work-stop-contract.md" },
  { unit = "stops", target = "#workflow" },
  { unit = "invalidation", target = "work-stop-contract.md" },
  { unit = "records", target = "context-views-and-shared-work.md" },
]
```

</details>
<!-- /ow:metadata -->

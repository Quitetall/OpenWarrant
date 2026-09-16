# OpenWarrant SDK contract

Normative companion to SAS RC.3 §§2, 6, 11–13. API spellings below are proposed
Rust-facing names; wire meanings are fixed by the named schema/profile. No new
function is claimed implemented by this document.

When representing context views, shared tasks or retained evidence, also apply
[context views and shared work](context-views-and-shared-work.md). For stop/change
records, apply [work-stop contract](work-stop-contract.md). The SDK provides typed
representations and checks explicitly supplied facts; it does not own SQL queries,
storage deletion, process cancellation or authority enforcement. Versioned codecs
and fixtures for these representations are Phase 1 SDK-19–SDK-24 deliverables.

## Public boundary

| Operation | Input and result | Responsibility boundary |
| --- | --- | --- |
| `parse_document` | Original byte slice plus limits → parsed document or located diagnostics | RC.3 footer plus explicit RC.2 adapter; preserve original bytes, metadata, unit IDs/kinds and UTF-8 byte spans |
| `validate_document` | Parsed document plus supported schema/extensions → validity report | Kind-specific required units, fields and local reference/condition syntax; external availability remains unresolved |
| `author_document` / `edit_document` | Typed fields or explicit field edits → new valid source bytes | Preserve untouched units; no hand-maintained digest/index fields; no hidden save or execution |
| `document.unit(id)` | Local stable unit ID → exact source slice or missing result | A heading rename does not rename the unit; no network resolution |
| `encode_canonical` / `digest` | Typed object and explicit domain → canonical bytes and digest | Reuse canonicalization and named domains; never rename old subjects |
| `check_records` | Supplied records, exact subject and explicit trusted facts → per-condition findings | Distinguish claims, observations, agent acts, human acceptance and qualification |
| `evaluate_readiness` / `evaluate_assurance` | Declared workflow/profile, normalized contract, evidence and trusted facts → scoped condition evaluation | Report qualification gaps separately from execution capability; no universal approval gate on unverified work |
| `decode_packet` / `check_packet_integrity` | Packet/manifest and explicitly supplied blobs → typed values and integrity findings | Check schema, hashes and declared ranges; label semantic coverage as not evaluated |
| Compiler adapter | Typed request and declared provider capabilities → validated response or error | Provider owns acquisition, closure, selection and package construction; SDK validates protocol boundaries |

All pure operations SHALL be callable without a database, model, service or
filesystem. I/O helpers are explicit wrappers. Each operation declares positive
resource bounds; oversized input refuses without truncating required content.
Malformed input returns named diagnostics and no partial value labeled valid.
Missing trust evidence is unknown/unverified, not implicitly trusted. A context-pointer
condition with missing task inputs remains unevaluated at document validation; its
runtime applicability evaluation belongs to the external compiler profile. Readiness
and assurance evaluate explicitly supplied record facts without hidden source resolution.

Document validity, reference resolution, context completeness, workflow readiness,
independent findings and assurance qualification SHALL be separate result fields
or separately typed reports. No boolean called `valid` may stand for all of them.

Absent OpenWarrant authorization or unmet qualification-only conditions SHALL NOT
become a universal execution refusal. Conditions identify whether they assess
qualification or enforce a named action gate. An explicit Warrant requirement for
verified preparation, a verified prerequisite or signoff before starting must
block that action until satisfied; choosing unverified mode cannot waive it.
Readiness results identify exact contract/profile, purpose, action scope and timing.
Contracts without these gates retain the prompt-only path. Real input availability, resource permissions
and harness limits remain separate. No SDK evaluation launches or prevents a tool
action by itself.

## Warrant scheduling and advisory estimates

Declared work dependencies SHALL be distinct from context pointers and current
blockers. A dependency names an exact predecessor Warrant contract and optional
milestone/stage, the required result, the affected successor action/stage and a
human-readable reason. An expected result digest may be omitted before the output
exists. Evaluation then requires one unambiguous supplied result and records its
exact digest through the selected fact; competing versions require explicit
selection. A supplied expected digest restricts matching to that exact result.
Required results distinguish implementation completion,
passing checks, human acceptance and a published contract. Completion does not
imply acceptance. Only the named successor scope waits; independent preparation
may continue. Qualification-only requirements do not become execution gates.

A pure SDK scheduling evaluator SHALL inspect an explicitly supplied finite graph
and caller-established result facts. Each fact binds the exact predecessor contract,
stage, required result and result/artifact digest. Missing or stale facts remain
unknown; failed facts remain unmet. Neither state satisfies the dependency. Inputs
that conflict, duplicate identities or contain dependency cycles SHALL be refused.
A report lists dependency-ready scopes and unsatisfied edges with reasons. Ready
means only that supplied dependency requirements are met: permissions, resource
limits, other action gates and current workflow blockers require separate checks.
The report SHALL NOT schedule processes, authenticate facts or issue assurance.

Workflow apps own acquisition and trust of facts, dispatch, agent availability,
current blocker tracking and dashboard presentation. They should show ready work,
predecessor/successor links and reasons for waiting. An acyclic dependency order
alone is not a time estimate or a measured critical path. Historical signed
contracts and context-reference semantics remain unchanged.

Difficulty MAY be recorded as low, medium, high or unknown, with a nonempty reason,
confidence (low, medium or high), estimator identity and estimate revision. This
is an advisory estimate, separate from size, elapsed time, spend and risk. Agents
may propose revised estimates with provenance; an estimate SHALL NOT grant
permissions, satisfy prerequisites, require a particular model or block execution.
Absence means no estimate, not low difficulty. Scheduling results do not change
when only a difficulty estimate changes.

The initial SDK slice uses typed Rust inputs, not a newly frozen wire schema.
Document metadata encoding, ingestion, CLI parity and workflow presentation need
explicit fixtures before being claimed supported. Existing F3 context dependency
fields SHALL NOT be repurposed as scheduling edges.

## Agent-act envelope: proposed additional wire type

Schema ID: `oh.war/agent-act/1.0.0-rc.3`. This is an additional envelope; existing
`oh.war/record/1.0.0-rc.2` kinds and legacy signature subjects are unchanged.

Required fields, with unknown core fields refused:

| Field | Meaning |
| --- | --- |
| `schema` | Exact schema ID above |
| `id` | Namespace-qualified record identity using document F1 identity grammar |
| `act` | `authorize` or `complete` |
| `subject` | Warrant identity, `contract_digest`, and `result_digest` for `complete`; no result required for `authorize` |
| `actor` | Namespace-qualified `id`, literal `kind: agent`, nonempty `role` |
| `policy_ref` | Exact policy identity and SHA-256 digest for a policy-based authority claim; may be null for unverified `complete`; required for `authorize` |
| `meaning` | Nonempty statement of the agent's act |
| `evidence_refs` | Unique record identity array; completion claims do not become check observations |
| `signature_ref` | Null for an unsigned draft, or identity/digest reference to a detached attestation |

Digests use `sha256:` plus 64 lowercase hexadecimal characters. The unsigned
payload comprises every field except `signature_ref`. Its proposed structured
digest domain is `oh.war/agent-act/1.0.0-rc.3`, with the F9 canonical wrapper.
The detached attestation SHALL bind that exact digest, signature scheme and key
identity. Attestation bytes are retained separately, avoiding a circular signature.
A hash or nonnull `signature_ref` is not proof of authentication. The caller's
explicit trust adapter must validate the signature and establish the agent/key
binding, and the applicable human-granted policy when authority under that policy
is claimed. No signer is invoked by parsing. A null policy on unverified completion
does not assert a permission grant. Ordinary execution/reporting need not create
an agent-act envelope at all; humans can report their unverified work too.

SDK validation may admit an unsigned draft as structurally valid while reporting
authentication as unestablished. An explicitly selected professional workflow
may require signed acts for its own admission; missing signatures do not block
the ungated prompt-only path. Explicit Warrant action gates still apply. The SDK SHALL refuse human-kind actors
in this envelope and SHALL NOT convert it into `human-acceptance`. Unknown signing
methods yield unsupported/unverified results, never successful authentication.
Wire fixtures/schema and positive/refusal tests are Phase 1 deliverables before
this proposal can be called implemented. They do not allocate real actors or keys.

## Assurance and secure human acceptance

SAS RC.3 requires an actual human signing act for the common assurance mark. The
SDK validates supplied exact subjects and profile criteria; signature authenticity
and human control come from a trusted adapter, not from JSON fields. A signer may
use SSH or another explicitly supported secure scheme. A service signing on an
agent's request does not establish that a human reviewed and signed the result.

The following outcomes SHALL remain representable:

- Ungated human/agent prompt-only work without prior OpenWarrant approval: execution and
  completion are allowed; result is complete and unverified without a mark.
- Unsigned draft alone: valid source, no execution proof or mark.
- Agent-signed complete result: attributable completion, possibly unverified,
  no human acceptance and no mark.
- Independent passing findings without human acceptance: checked result, no mark.
- Human signature without required passing evidence: accepted statement, no mark.
- Exact evidence plus authenticated human acceptance and every baseline criterion:
  eligible for the workflow to issue the mark for that exact result.

Qualification SHALL evaluate the applicable authorization/start conditions and
their required timing against actual records. Later final-result acceptance can
qualify prototype work without inventing prior authorization. A condition that
explicitly requires a pre-work event cannot pass based on a later signature.
An unmet qualification-only condition refuses the corresponding mark, not ordinary
execution. An enforced action gate also blocks its named action until satisfied.

Batch human acceptance MAY cover an exact release-review manifest containing
multiple Warrant/result/profile/evidence subjects. Authenticate the human act and
explicit coverage of every included subject; do not infer batch coverage from a
free-text release name or widen a legacy single-subject signature. Batch wire
encoding and signature vectors must be fixed before support is claimed. Every
member has its own assurance result; missing support cannot inherit another's pass.

## Work-stop response and tracker handoff

The SDK SHALL represent complete/unverified and complete/verified using the same
completion meaning with separate qualification standing. Missing qualification
records do not leave ordinary completion pending or require a signature to record
it. A supplied agent completion can remain unsigned and make no authority claim.

Every work stop has an exact event/result identity and scope, configured safeword,
pointer to a deterministic progress overview, implementation notes, generated
document trail and next steps. The overview has tracker source/revision/as-of
identity and configurable pending-work lists, progress indicators, statistics and
data views. A feature stop can leave its Warrant in progress. Its signal must
identify the completed feature rather than falsely completing the Warrant.

Chat defaults to pointers. The exact safeword is the first line of the normal
work-stop response; profile configuration controls inline fields, note length and
token budget. A minimal profile may return safeword plus the scoped overview link,
with notes, trail and next steps in that linked document. A richer profile may
include short excerpts. No profile requires pasting the full overview into chat.
The machine record retains scope, state and source identity even when inline
wording is reduced. Configuration must not hide qualification or discard required
information from the referenced package.

Wire encoding and response profiles must be fixed with fixtures before support
is claimed. Tracker I/O and state mutation belong to the workflow adapter;
deterministic rendering/compilation belongs to its view provider. Agent-authored
notes are labeled separately from generated facts. Pure SDK operations inspect
supplied references, facts and hashes and report their trust/semantic limits.

Validate that the referenced projection acknowledges the named completion/result, that its
scope and revision are explicit, and that qualification standing is independent.
Do not infer live freshness or authoritative tracker origin merely from an
agent-authored field; the consumer establishes provenance through its tracker
adapter. Retries use the same completion-event identity. A delivery/sync failure
is separate from work state and cannot fabricate a successful tracker update.

Safeword wording is configurable by the workflow, not a universal standard token.
The provisional reference default is `WORK_DONE`, suitable for any declared work
scope. Emit it in the control position only on that scope's completion. Ordinary
chat may mention the same word; no global word ban applies. A word alone cannot
mutate records, qualify a result or authenticate its actor. The user's existing Stable
publication safeword remains specific to that separate release event.

## External compiler adapter

The SDK SHALL expose explicit version/capability negotiation and typed requests
and responses for the retained context profile. Requests contain captured source
metadata/bytes or explicit source references, task/role, selection inputs, access
basis and resource bounds as specified by the profile. Providers return their
identity/build, exact basis, diagnostics, omissions, context completeness, readiness
and requested outputs. A caller selects the provider; no ambient fallback silently
changes semantics, models or authority. Unsupported profile/capability, unavailable
provider, malformed response and mismatched basis are distinct errors.

An adapter MAY delegate full semantic package validation to a qualified provider.
It SHALL identify that provider and report its evidence separately from local
structural/integrity checks. Fake providers exercise response handling only.
LAMU must pass shared interoperability cases with its real implementation before
its profile is advertised as supported. Production ABI/transport bindings and
capability schema are frozen with concrete fixtures in Phase 2; no method names
above are claims of an installed RPC or CLI endpoint.

## CLI and reference helpers

Phase 1 SHALL expose its scoped SDK operations after direct-library proof. Proposed
namespace: `war document` for author/edit/check, `war record` for supplied record
inspection, and `war packet check` for explicitly bounded integrity inspection.
These spellings are reserved design examples, not commands to run today. Exact
command/help/JSON/exit contracts are part of the Phase 1 implementation Warrant.
Noninteractive mode never prompts; one structured result goes to stdout. Interactive
editing may follow the noninteractive path but must preserve prior bytes on cancel.
SDK consumers need no CLI shell process for in-process use.

A small reference compiler adapter demonstrates an external call and validates its
response. It SHALL NOT contain production dependency traversal, context ranking,
master assembly or hidden model calls. Semantic compilation remains provider-owned.

## Supplied-record SDK profile (OW-WAR-0083)

The Phase 1 record codec implements F8 `oh.war/record/1.0.0-rc.2` and the
agent-act profile above. The candidate workflow encoding is
`oh.war/workflow-record/1.0.0-rc.3`: an object with `schema`, `id`, and
`payload: {kind, value}`. Unknown fields and duplicate JSON keys refuse. Its
structural schemas and executable inputs live in `conformance/sdk/records/`.
Schemas describe structure; SDK checks additionally enforce relationships, exact
subjects, resource bounds and trust separation. This draft profile is not a
claim that RC.3 has been accepted or published.

| Payload kind | Preserved facts |
| --- | --- |
| `context-view` | Source digest, document maturity, work state, qualification claim, code-derived or approved-document origin |
| `shared-contract` | One contract digest; each participant's project, scope and exact basis |
| `evidence-availability` | Exact evidence reference; present, absent, deleted or restored state; retained digests, history references and affected assurance |
| `stop` | Exact subject, class, scope, scope ID, optional parent, cause, work state and observed worker state |
| `work-change` | Work/harness class, old/new basis, affected scope, application boundary, choice/fencing/context references and limits |
| `overview` | Tracker identity, revision, observation time and exact completion events with qualification, evidence, notes, trail and next steps |
| `handoff` | Event identity, exact overview reference and URL, configured safeword, qualification, evidence and required pointers |
| `review-manifest` | Exact result subjects, each selected profile and evidence set |

`program` encodes the named complete-stop scope. Work states include `pending`,
`in-progress`, `complete`, `blocked`, `failed`, `cancelled` and `unknown`.
Worker states include `running`, `stop-requested`, `stopped` and `unknown`.
A work stop requires complete work and an exact result. An interruption may
preserve earlier completion, but cannot produce a new completion signal.
These records are bounded SDK inputs, not a replacement for the richer runtime
journal required by the work-stop contract.

### Trust is an explicit input

Record content cannot authenticate itself. `TrustedRecord` binds caller-established
facts to exact original bytes, record ID and actor identity/kind. Authentication,
observed execution, secure human signing, observation time and support for a named
assurance criterion are separate facts. An authenticated claim is not an observation.
The caller must establish these facts through its trusted adapters; the SDK never
accepts them from the record's claimed provenance or signature reference.

Every evidence, policy, isolation and observation-artifact reference must resolve
within the supplied records before dependent support can be established. External
artifacts require supplied authenticated receipt records; unresolved references
remain unknown. Cyclic evidence cannot establish itself. No network lookup occurs.

Readiness retains the exact action, stage and selected conditions, including timing
and whether each condition gates action or only qualification. No selected action
gate means no gate is invented. Unsupported permission constraint expressions remain
unknown, rather than being ignored. Delegation must match an exact secure human
policy record and permitted act; automation cannot grant itself policy-edit authority.

Assurance requires the normalized contract and exact Warrant source descriptor,
expected scopes/check digests, independent observations and secure human acceptance.
The baseline cannot be removed by selecting an empty condition list. Evidence for
protected expectations, scope permission, adequacy, preservation and isolation needs
explicit caller-established criterion support, not an arbitrary passing check.
Additional profile conditions strengthen this assessment. The result is eligibility,
ineligibility or unknown; this helper issues no assurance mark.

A batch review manifest establishes human acceptance coverage only. Each exact
member still needs its own assurance evaluation; adding a member or changing its
profile/evidence invalidates the supplied signature coverage. A release name alone
cannot establish acceptance.

### Completion and recovery helpers

A confirmed handoff requires caller-established tracker identity, revision, exact
bytes and URL. The overview must acknowledge the exact event, result and scope,
qualification and required pointers. Minimal output is the configured safeword and
overview URL; richer output adds notes. Missing synchronization leaves completed
work intact while response delivery remains pending. Neither form completes a
parent scope. Replayed IDs must retain exact bytes.

Resume checks consume explicit exact-change facts for choice, fencing, context,
limits and the selected boundary. Work changes require fencing; harness-only changes
apply before the next tool action. References alone do not establish these facts.
Evidence availability checks retain history and distinguish missing bytes from a
retained exact duplicate. These helpers neither stop processes nor delete storage,
verify cryptographic signatures, dispatch agents or perform tracker I/O. Those
integration proofs belong to the workflow phase.

## Explicit preservation and successor profile (OW-WAR-0084)

The candidate `oh.war/preservation/1.0.0-rc.3` envelope retains an explicit regular-file
inventory, not a rewritten historical Warrant. Its fields are `schema`,
`source_dialect`, `adapter_version`, `entry`, `inventory` and `files`.
`inventory` entries contain original relative `path`, raw SHA-256 `digest` and
`bytes`; `files` maps those exact paths to byte arrays. Version
`ow-sdk-preservation/1` admits explicit `legacy-warrant-v1`, `rc2` and `rc3`
dialects. Markdown is never used to guess a dialect. Unsupported editions refuse.

Import checks exact inventory coverage and bytes, path safety, declared dialect
and parser syntax. It preserves original schema/identity, historical state fields,
signature subjects and original JSON as bytes. The legacy Warrant adapter uses
the existing manifest validation and restricted Markdown atom parser. Bound or
missing atoms are reported as unsupported; they are not fetched or invented.
Historical filename colons remain literal; rooted, drive, traversal and ambiguous
separator paths refuse. Filesystem callers must reject links and special files
before supplying regular-file bytes, and choose a fresh destination for export.

The preservation report separates inventory integrity from historical closure,
authenticity and qualification. It never establishes external historical closure,
acceptance meaning, signature validity or a new assurance mark. A present legacy
resolution is reported as `resolved-record-present`, with its original outcome,
standing and contract fields retained separately. This label is not a new
resolution. Unknown historical meaning remains in the retained bytes and is not
silently translated into the new model.

Export/import/export retains deterministic envelope output and identical original
blobs. Wire, file-count, per-file and total limits apply before blob materialization;
output is bounded while serialized. A refusal returns no valid partial bundle.
These pure calls never write or replace original files.

A successor mapping binds distinct source identities and exact predecessor and
successor entry digests, plus each declared same-path replacement's old/new digest.
Both input inventories remain recoverable. The mapping transfers no authority,
signature, acceptance or qualification and creates no correction fanout. It is
provenance for new work, not authorization to modify legacy pinned files.
Only explicit relevant prerequisites gate their named successor action; the
existing dependency evaluator does not make unrelated unresolved records blockers.
Retained-evidence assurance gaps use the supplied-record availability helpers.
These SDK checks do not replace migration workflow, storage retention or signing.

## Offline CLI transport profile (OW-WAR-0087)

The candidate `war sdk --request <file|->` shell exposes the Phase 1 document,
record, integrity and preservation SDK operations without repository discovery.
The request schema is `oh.war/sdk-request/v1`; the response uses the existing
`oh.war/report/v1` envelope. See the operation and encoding table in
[the CLI profile](../../../../conformance/sdk/cli/README.md).

Embedded source and record bytes remain explicit. A successful evaluation can
report unknown, unmet or conditionally eligible standing; exit zero alone is not
readiness or qualification. Caller-supplied trust and policy facts remain
unauthenticated assumptions. No signer, provider, model or worker is invoked.
Optional output writes a new JSON result file without replacing existing bytes;
it does not edit the input document or persist an authority act. The CLI crate's
file-backed driver compares public SDK outcomes and tests named refusals and
I/O preservation. Interactive authoring and workflow execution remain later work.

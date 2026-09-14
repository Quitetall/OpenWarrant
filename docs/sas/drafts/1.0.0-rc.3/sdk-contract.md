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
| `parse_document` | Original byte slice plus limits → parsed document or located diagnostics | Preserve original bytes, metadata, unit IDs/kinds and UTF-8 byte spans |
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

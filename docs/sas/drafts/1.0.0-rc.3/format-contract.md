+++
schema = "oh.war/document/1.0.0-rc.2"
kind = "context"
id = "openwarrant:format-contract"
revision = 3
title = "SDK document format and external compiler profile"
state = "proposed"

[[dependencies]]
unit = "units"
target = "#source"

[[dependencies]]
unit = "pointers"
target = "#source"

[[dependencies]]
unit = "pointers"
target = "#units"

[[dependencies]]
unit = "conditions"
target = "#pointers"

[[dependencies]]
unit = "request"
target = "#source"

[[dependencies]]
unit = "request"
target = "#records"

[[dependencies]]
unit = "request"
target = "#digests"

[[dependencies]]
unit = "selection"
target = "#pointers"

[[dependencies]]
unit = "selection"
target = "#conditions"

[[dependencies]]
unit = "package"
target = "#request"

[[dependencies]]
unit = "package"
target = "#selection"

[[dependencies]]
unit = "package"
target = "#digests"

[[dependencies]]
unit = "records"
target = "#digests"
+++

<!-- ow:unit context binding -->
# SDK document format and external compiler profile

Status: normative compatibility profile carried by [SAS 1.0.0-rc.3](WAR_Software_Architecture_Specification.md).
Wire identifiers remain RC.2; the program SAS edition does not rename existing
payloads or signatures. Source framing/types/units and F8/F9 record/digest codecs
are SDK responsibilities. F3/F4 syntax is validated by the SDK; cross-source
resolution and condition evaluation belong to the compiler provider. F5–F7
construction, selection, semantic coverage checking and budget/cache behavior
belong to LAMU or another conforming provider. The SDK exposes those wire types
and structural/integrity checks without claiming full semantic conformance.
These are required profile meanings, not proof that installed `war` implements
them. No model call is required by this deterministic profile. The new agent-act
envelope is specified separately in [SDK contract](sdk-contract.md); it does not
reinterpret this profile's existing `human-acceptance` records.

<!-- ow:unit source binding -->
## F1. Source framing and metadata

A new source is UTF-8 without BOM or NUL. It begins with a line containing exactly
`+++`, followed by a TOML header, another exact `+++` line, then Markdown units.
Both LF and CRLF are accepted and preserved in source blobs. A framing line is
recognized after removing only its line terminator. An unterminated header fails.
RC.2 forbids TOML multiline strings so a delimiter cannot occur inside one.
Body Markdown can contain code and quotations; neither is executed.

Header syntax follows [TOML 1.0.0](https://toml.io/en/v1.0.0), limited by the types
below: strings, booleans, integers from 0 through 9007199254740991, arrays, and
tables. Floats, dates/times, and null are not supported. A schema's nonempty array
or positive-integer requirement narrows those general types. Duplicate keys fail.
The parser SHALL preserve original bytes separately from parsed field values.

| Field | Type and meaning | Required/default |
| --- | --- | --- |
| `schema` | Exact string `oh.war/document/1.0.0-rc.2` | Required |
| `kind` | `warrant`, `sas`, `adr`, or `context` | Required |
| `id` | Stable document identity; ASCII namespace-qualified string | Required |
| `revision` | Positive integer, author revision label | Required |
| `title` | Nonempty single-line string, at most 256 Unicode scalar values; no control characters | Required |
| `state` | `draft` or `proposed`; author intent only | Required |
| `alias` | Nonempty human name, not identity | Absent |
| `scope` | Table with optional `subsystems` and `paths`, each an array of strings | Absent/unknown |
| `context` | Ordered array of pointer tables, F3 | Empty |
| `dependencies` | Ordered array of dependency tables, F3 | Empty |
| `conflicts` | Ordered array of conflict tables, F3 | Empty |
| `extensions` | Table keyed by namespace-qualified extension ID | Empty |
| `requires_extensions` | Unique array of extension IDs needed to interpret the source | Empty |

An identity matches `[a-z][a-z0-9-]*:[A-Za-z0-9._:/-]+`. Identity comparison is
case-sensitive. Examples use reserved `example:` IDs; they are illustrative, not
registry allocations. Tools SHOULD generate `urn:uuid:<UUIDv7>` identities for
new shared documents, without requiring a registration service. A captured set
cannot contain two different bytes under the same identity and revision unless
explicitly treated as competing versions; complete projection then requires
resolution. A new digest never inherits approval because its revision label matches.

Unknown core fields, including misspellings, fail validation. Unknown optional
extensions are preserved and diagnosed but have no authority or selection effect.
An unsupported `requires_extensions` item blocks semantic compilation. Extensions
cannot redefine core fields or smuggle unsupported required behavior as optional.

<!-- ow:unit units binding -->
## F2. Source units and minimum documents

A source unit is a marked span within one document, not a whole legacy atom file.
Canonical definitions are in SAS §3; these wire rules give their exact encoding.
A unit starts with this exact, unindented marker line outside a fenced code block:

```markdown
<!-- ow:unit outcome binding -->
## Desired outcome

The user can request a password reset.
```

Grammar: `<!-- ow:unit <unit-id> <kind> -->`, where unit-id matches
`[a-z][a-z0-9-]{0,63}` and kind is `binding` or `background`. The next line must
be an ATX heading (`#` through `######`, one space, nonempty title). Unit IDs must
be unique within the document; heading titles may repeat. A rename preserves the
unit ID. Malformed marker-looking lines outside fences fail rather than silently
turning a binding unit into background.

A unit's exact text range starts immediately after its marker's line ending and
ends at the next marker's first byte or EOF. It includes its heading, nested
headings, Markdown, blank lines, and original line endings. The marker itself is
metadata, not unit text. Source maps use zero-based UTF-8 byte offsets `[start,end)`;
no Unicode or newline normalization is applied to binding slices.

For marker recognition only, a fence is an unindented line starting with at least
three identical backticks or tildes; closing fence uses the same character with
at least the opening count and only trailing spaces/tabs. An opening backtick
info string may not contain backticks. Markers inside a fence are ordinary text.
An unclosed fence is an error. Indented code, list items, blockquotes, and HTML
rendering never activate a marker: only the exact column-zero line does. This is
an explicit OpenWarrant structural scanner, not inference from rendered Markdown.
Content before the first unit, after the header, must be whitespace only.

| Kind | Required nonempty units | Meaning |
| --- | --- | --- |
| Warrant | `outcome`, `scope`, `context` | Intended behavior; included/excluded work; known basis and unknowns |
| SAS | `outcome`, `scope`, `context` | System goals; architecture boundary; governing basis |
| ADR | `context`, `decision`, `consequences` | Problem and options; chosen/proposed rule; effects and tradeoffs |
| Context | At least one unit | Reusable rules, glossary, or background |

Required units are `binding`; an ADR may have proposed binding text without
accepted authority. `context` may say no sources are known or name an unresolved
question; that is syntactically valid and grants nothing. “Nonempty” means content
other than the heading and whitespace. Additional units may have either kind.
A selected Warrant always includes all its binding units, including optional
`constraints`, `acceptance`, `outputs`, `stop`, or `plan` units if authored.
A step in a binding unit is a requirement; authors put flexible implementation
suggestions in a background unit. Documents need no fixed number of sections.

A source is valid independently of cross-source availability. Structural reference
syntax is checked at validation; resolution/completeness is checked at capture or
projection. A missing required target cannot yield a complete execution packet.

<!-- ow:unit pointers binding -->
## F3. Pointers, dependencies, and captured references

Example header entries:

```toml
[[context]]
id = "signup-rule"
target = "architecture.md#activation"
required = true
when = { stage = ["implementation", "verification"], subsystem = ["identity"] }

[[context]]
id = "design-history"
target = "rationale.md#tradeoffs"
required = false

[[dependencies]]
unit = "activation"
target = "architecture.md#inactive"
```

Pointers have exactly `id`, `target`, `required`, and optional `when`. Pointer IDs
use unit-ID syntax and are unique per document. `required=true` includes matching
content and its dependencies. `required=false` creates a catalog entry; bytes are
included only when `include_optional=true` is supplied to compilation. Required
context is never weakened by a false flag on a different pointer. Every pointer
in every captured document is evaluated, so governing sources can route their
own rules even when the Warrant does not repeat them.

Dependencies have exactly `unit` (local unit ID) and `target`. They are unconditional
when that unit is selected and transitively require their targets. Conditions
belong to pointers, not dependency edges. Conflicts have exactly `unit` and
`target`; a conflict is symmetric when both targets are selected. The originating `unit` must exist locally; the target may be a unit in another
captured source. Cross-document dependency edges are explicitly allowed. Target
existence is a resolution concern.

Targets are `<path>#<unit-id>`, `<path>#*`, `#<unit-id>`, or `<path>` for a raw
whole-file reference. `#*` means all units plus header semantics of a parsed
source. An omitted path means this source. Paths are capture-root-relative, not
relative to each document's directory. They use `/`, contain nonempty segments,
and must not start with `/`, contain `.` or `..` segments, `\\`, `:`, NUL, `#`,
or control characters. There is no URL decoding, shell expansion, globbing,
network fetch, or implicit `latest`. An opaque file cannot be selected by unit ID.

Authors normally write paths and unit IDs. Capture resolves them once and emits a
lock table: source path, ID/revision when parsed, raw SHA-256, byte length, holder
locator, and metadata basis. A source path may have only one active snapshot per
capture. The original immutable lock table is retained with approved work; a
later capture of the same path is a different basis. No hand-copied digest is
required in the author's pointer.

A bound unit reference in compiler output has exactly:

```json
{"source_digest":"sha256:<64 lowercase hex>","unit":"activation","start":0,"end":1}
```

Offsets here illustrate types only. Actual offsets must select the named unit in
that source, not arbitrary bytes. Whole-file references use `unit:"*"`, start 0,
and end equal to the source length. The source table carries path, identity,
revision, and holder; consumers do not infer authority from a hash or path.

<!-- ow:unit conditions binding -->
## F4. Conditions and scope inputs

`when` is a nonempty table containing any subset of `stage`, `subsystem`, and
`path`. Each value is a nonempty array of nonempty strings. There are no nested
operators in RC.2. Entries within a field are OR; different fields are AND.
A pointer without `when` is unconditional. Unknown keys or malformed values fail
with `condition-invalid`; they do not activate conservative inclusion.

Selection inputs are `stage` (a nonempty string), `subsystems` (array of names),
and `paths` (array of concrete capture-relative paths), each optional. Stage and
subsystem matching are exact, case-sensitive string matching. Subsystem matches
if any declared subsystem occurs in its condition list. No taxonomy is inferred.
Path matches if any declared path matches any condition pattern.

Path patterns follow F3 path rules except for wildcards. `*` matches zero or more
characters within a segment; a segment equal to `**` matches zero or more complete
segments. `**` embedded within another segment, `?`, brackets, and brace patterns
are invalid. Matching is case-sensitive, anchored to the whole path, and does not
consult the filesystem. No regex expressions or platform glob defaults are used.

Missing input means UNKNOWN. An explicitly supplied empty subsystem/path list is
known and yields FALSE. Each field is evaluated and reported; AND is FALSE if any
field is false, TRUE if all are true, otherwise UNKNOWN. Thus a known false stage
can make a pointer false despite an unknown path. A required UNKNOWN pointer
includes its target and dependencies, recording the missing inputs and original
condition. Missing required content still blocks. Optional UNKNOWN pointers remain
catalog entries unless optional inclusion was requested. When optional content is
requested and no target bytes are available, keep it absent with a warning. If the
target exists but its mandatory dependency is missing, the requested inclusion
cannot produce a complete package.

The caller selects a Warrant snapshot plus input overrides. Unspecified subsystem
and path inputs use that Warrant's `scope` metadata if present, otherwise remain
unknown; stage has no implicit value. Explicit overrides name their provenance in
the request and do not mutate the source. They are declared scope, not proof of
all files an agent will touch. Authorization checks belong to the consuming policy.

<!-- ow:unit request binding -->
## F5. Captured input and deterministic compiler contract

The pure library receives typed values corresponding to this JSON request shape.
All top-level fields below are required, with explicit empty arrays/maps permitted.
`mode` is `task` or `master`; `task` is null for master mode:

```json
{
  "schema":"oh.war/compile-request/1.0.0-rc.2",
  "mode":"task",
  "compiler":"<implementation name and exact build digest>",
  "sources":[],
  "task":{"source_digest":"sha256:<hex>","role":"implementation","stage":"implementation"},
  "selection_inputs":{},
  "policy_facts":[],
  "records":[],
  "options":{"include_optional":false,"entry_token_budget":8192,"token_method":"utf8-bytes-div4-ceil","limits":{},"access":{"allowed_source_digests":[],"basis_refs":[]}}
}
```

Each source has `path`, `source_digest`, `byte_length`, `holder`, `metadata`, and
optional `document` (`id`, `revision`, `schema`, `kind`). `holder` is an object with
`kind` (`local`, `git`, `external`, `example`) and `locator` string; optional `commit`
records an exact Git revision. The caller separately supplies a blob map keyed by
raw source digest. `metadata` has `trust`, `classification`, `taints` (string array),
`authority` (`unestablished` or `established`), and `basis_refs` (record IDs).
Trust/classification labels are opaque policy labels; no lexical ranking applies.
A parsed source's document fields must agree with its bytes. `sources` are sorted
by path. All source paths, including ones later omitted, participate in the basis.

`task.role` is `preparation`, `implementation`, `verification`, or `human-review`.
`task.stage` is optional; omission is unknown. In task mode the source must be
a Warrant. Master mode requires `task:null`, selects all captured units into an
assembly, and returns readiness `not_evaluated`; it produces master IR/views, not
a task package. The mode is part of the captured basis. A master is not an
execution readiness claim.

`selection_inputs` may contain stage/subsystems/paths and an `origins` map from
field to provenance string. Provided stage overrides task.stage, with both retained
in the manifest. `policy_facts` is an array of IDs resolving into `records`.
`options.access` is a required caller-supplied object with
`allowed_source_digests` (unique raw digest array) and `basis_refs` (record IDs).
Every blob to be disclosed must be in that allowlist; an absent grant denies it.
The compiler reports access as supplied policy facts, not independently verified
identity. The caller is responsible for establishing those grants at its trusted
I/O boundary. Empty basis_refs are allowed for a content-only example, with the
limitation explicitly reported; they establish no execution permission or mark.
Both access arrays may be empty. An empty allowed_source_digests array allows
no blob disclosure; it is not an unrestricted grant.
No unspecified ambient configuration enters the pure result. Unsupported schema
versions, duplicate source paths/IDs where ambiguous, or mismatching blob lengths
or digests are errors.

Output has `schema`, `document_validity`, `context_complete`, `readiness`, `ir`,
`diagnostics`, and optional `package`. Validity is `valid` or `invalid`; readiness
is `ready`, `blocked`, or `not_evaluated`, always with basis/limitations. Preview
may return valid IR and diagnostics without a package. Complete package output
is allowed only when context_complete is true. Compiler success never attests
human identity, executed checks, or sandbox protection it did not observe.

Default limits are 8 MiB per source, 256 MiB total input bytes, 4096 sources,
65536 units, 262144 dependency edges, 1 MiB of header per source,
64 MiB of rendered entry bytes, and 512 MiB of package bytes. Render/output limits
are checked incrementally as well as at publication. Values are
inclusive. Custom positive limits must be supplied, recorded, and checked before
allocation/traversal. Limits are implementation resource controls, not permission
to truncate. Budget estimates are `ceil(UTF8 byte length of ENTRY.md / 4)` for the
named baseline method; they are estimates, not tokenizer output. Other methods
need separate versioned identifiers and conformance fixtures.

<!-- ow:unit selection binding -->
## F6. IR, selection, and diagnostics

IR SHALL preserve source metadata, exact unit ranges and kinds, pointers,
dependencies, conflicts, source/contract records, and selection results. Document
order is source path order then byte-offset order. Pointer evaluation order is
source path then pointer ID. Output set-like arrays (selected units, omitted
pointers, diagnostics, sources, reasons) use lexicographic tuple order as follows:
unit `(source_digest,unit)`, pointer `(source path,pointer ID)`, diagnostic
`(code,source path,unit-or-empty,pointer-or-empty)`, reason `(kind,source path,id)`.
Here reason tuples are inclusion reasons in `binding_context`. Catalog
evaluation reasons are diagnostic strings, deduplicated and sorted
lexicographically. Authored plan/step arrays keep their explicit order.

The `task.role` also selects a fixed action meaning. Warrant outcome, scope,
constraints, and expected product outputs remain identical; role changes what
the recipient does with that subject. The entry SHALL display the corresponding
action and stopping condition before the common task brief:

| Role | Action and role output | Stop condition |
| --- | --- | --- |
| preparation | Draft/reuse the Warrant and proposed acceptance fixtures; report unknowns | Proposal ready for review, or a blocking question is recorded |
| implementation | Produce candidate artifacts and evidence-backed claims within the contract | Candidate ready for independent verification, or affected work is blocked |
| verification | Inspect the exact candidate independently, rerun required checks, return bounded dispositions and evidence | Findings cover required obligations, with unknown/failure preserved |
| human-review | Review outcome, independent findings and risks; inspect code as needed; request accept/reject/revision through the authorized workflow | An attributable decision is recorded by that workflow, or review remains pending |

These actions are compiler instructions, not authority grants. Product outputs in
`brief.outputs` identify the subject to deliver or assess, not permission for a
verifier to modify code or for an agent to perform a human act. Required role
evidence must be supplied in the source set/records; absent evidence blocks the
corresponding readiness evaluation rather than inventing a favorable verdict.

Selection begins with all binding units of the task Warrant, plus true/unknown
required pointers from every captured source. It expands required dependencies
to a fixed point. With include_optional, true/unknown optional pointers add their
content and dependency closure but retain optional origin labels. Required
unconditional roots and required closure win over other reasons for omission.
A dependency target classified as background is included verbatim as required
context; its authoring kind does not cancel the dependency. Required opaque
files are supplied as whole blobs and linked from the entry view.

Self/long dependency cycles terminate by visited `(digest,unit)` identity and emit
`dependency-cycle` with members; all members are included once. A cycle itself is
a warning. Missing member, explicit conflict, incompatible required revisions,
or access denial is an error. Required remote missing targets are not silently
classified as optional. Incomplete optional targets produce a catalog diagnostic;
when requested for inclusion, their missing required dependencies prevent a complete
package. Arbitrary prose contradiction detection is outside deterministic coverage.

Diagnostics have `code`, `severity` (`error`,`warning`,`info`), `message`, optional
`source_path`, `unit`, `pointer`, and `details` object with machine-readable basis.
Codes required for RC.2 include: `source-invalid`, `schema-unsupported`,
`extension-required`, `unit-duplicate`, `target-invalid`, `target-missing`,
`target-ambiguous`, `digest-mismatch`, `condition-invalid`, `applicability-unknown`,
`dependency-cycle`, `binding-conflict`, `access-denied`, `budget-exceeded`,
`resource-limit`, `readiness-blocked`, `record-untrusted`, `package-invalid`.
No error is converted into PASS to produce output. Unknown applicability is a
warning with conservative inclusion; absent required execution permission blocks
readiness independently.

<!-- ow:unit package binding -->
## F7. Offline package and integrity

The standard package is a directory of regular files:

```text
ENTRY.md
packet.json
manifest.json
blobs/<64-lowercase-hex>.bin
```

All JSON files use [RFC 8785 canonical JSON](https://www.rfc-editor.org/rfc/rfc8785)
with no terminal newline. The entry uses LF framing; embedded unit text retains
its source bytes, including any CRLF. Source blobs are byte-exact. The directory
name, file mtimes, and filesystem order do not affect identity. No archive format
or online resolver protocol is required. Duplicate/case-colliding paths,
symlinks, traversal, unknown top-level entries, and unlisted files are rejected.

`packet.json` has these required fields. The complete
[reference packet](examples/expected/signup-packet/packet.json) supplies a concrete
instance; [packet JSON Schema](schemas/packet.schema.json) fixes structural types.
Prose adds cross-field, provenance, and semantic requirements that JSON Schema
does not establish by itself:

| Field | Content |
| --- | --- |
| `schema` | `oh.war/context-packet/1.0.0-rc.2` |
| `compiler` | Exact implementation identifier/build digest |
| `basis_digest` | F9 digest of the captured request excluding blob bytes (sources carry their raw digests) |
| `task` | Source digest, role, optional stage |
| `brief` | Object with required arrays `outcome`, `scope`, `context`, `constraints`, `outputs`, `stop`; each contains inline F3 bound unit references, empty if no such unit |
| `binding_context` | Every selected bound reference, in F6 order, with its source kind and required/optional origin |
| `reference_catalog` | Every evaluated pointer: source, ID, bound target or unresolved target, condition, result, reasons, `supplied`, `selected`, and `required` booleans |
| `sources` | F5 source entries needed for included content or provenance; missing optional targets may be catalog strings only |
| `selection_inputs` | Effective inputs and origins |
| `diagnostics` | F6 records |
| `readiness` | Object with `value` enum, `basis` record-ID array, `limitations` string array; never inferred from context completeness |
| `coverage` | Literal `declared-inputs-and-dependencies`; no claim about undeclared rules |
| `accounting` | `entry_bytes`, `estimated_tokens`, `method`, `budget`, `package_payload_bytes` |

Each binding_context entry is `{ref,kind,required,reasons}` where `ref` is the literal object key containing the inline F3 bound reference,
`kind` is `binding`, `background`, or `opaque`, and `reasons` uses F6 tuples as
objects. `sources` includes all F5 source entries. Package blobs include every parsed
source whose routing declarations were evaluated, every selected source, and all
required evidence. This lets an offline checker reconstruct the declared routing
basis. Unselected opaque sources may have absent blobs; their metadata/digests
remain in the basis. Full parsed-source preservation can make transfer size larger
than the entry projection; v1 optimizes initial agent context, not guaranteed
package transfer size. Catalog `supplied` means target bytes exist in package,
even if not selected for ENTRY; a separate `selected` boolean states that choice.
Catalog `reasons` is an array of diagnostic strings explaining pointer evaluation
and availability; it is distinct from the structured inclusion-reason objects
in `binding_context`. Both are required to retain their stated meanings.

ENTRY.md renders, in order: `# <title>`; role/stage, F6 role action/stop condition, and coverage/readiness notice;
`## Task brief` with the six brief lists and references; `## Binding context` with
selected text units in F6 order, each preceded by source path, digest and unit ID;
`## References` with catalog entries; and `## Diagnostics`. Required text is
embedded exactly once in Binding context; brief lists point to those units inside
the package. Opaque data is linked to its blob with type/length/digest, not decoded
as instructions. Original source marker lines are excluded. The checked-in ENTRY example defines reference framing for that task. General
framing substitutes title/role/stage/values in that order; a missing stage renders
`unknown`. Escape Markdown metacharacters in generated labels and render untrusted
strings as data; binding unit bytes remain untouched. Empty brief lists render
`not supplied`, never an invented permission or criterion. General renderings and
all four roles require additional golden fixtures before Phase 2 conformance;
exact embedded slices and semantic membership bind in Phase 1.

`manifest.json` has `schema:"oh.war/context-package/1.0.0-rc.2"`, `files` (sorted
array of `{path,sha256,bytes}` for ENTRY, packet, and blobs), `basis` (complete F5
request without blobs), and `root_digest`. The root is F9's package-domain digest
of this manifest with `root_digest` omitted. `manifest.json` SHALL NOT be listed in its own
files array; its size/digest are excluded from the root preimage. `sha256` values use the prefix `sha256:`. All required blob references must
have a listed file. Check exact file set, byte lengths, hashes, root, source/unit
ranges, and reference closure. Hash-valid does not mean semantically conforming:
recompute selection from every captured parsed source and validate membership
against the captured basis. A missing routing-source blob makes the package
invalid; missing bytes for an omitted opaque candidate are permitted. The release
fixture suite must include a self-consistent but semantically incomplete package.

To avoid accounting cycles, `package_payload_bytes` is defined as ENTRY bytes plus
unique blob bytes only; it excludes packet/manifest JSON. The physical total is
reported separately by the consumer after writing the package. Neither number
is presented as model-token usage. No field depends on its own serialized size. The compiler SHALL first render
ENTRY in memory (subject to resource limits), then compute its budget estimate.
Only an in-budget result may be published as a package. Over-budget requests return
`budget-exceeded`, blocked readiness with a reason, optional diagnostic IR, and
no package. No refused package or partial destination is published. Token budget
is not a pre-render memory limit; resource limits remain independently enforced.

<!-- ow:unit records binding -->
## F8. Record meaning and supplied policy facts

Phase 1 consumes a bounded record envelope. It does not implement a signing,
identity, execution, or policy service. Records supplied as JSON have these fields:

```json
{
 "schema":"oh.war/record/1.0.0-rc.2",
 "id":"example:record-1",
 "kind":"claim",
 "subject":{"warrant":"example:signup","contract_digest":"sha256:<hex>","result_digest":"sha256:<hex>"},
 "actor":{"id":"example:agent","kind":"agent","role":"performer"},
 "policy_ref":null,
 "evidence_refs":[],
 "payload":{},
 "provenance":{"source":"example:input","authenticity":"unverified"}
}
```

`id` uses F1 identity syntax. `kind` is `claim`, `observation`, `inference`,
`judgment`, `verification`, `permission`, `policy-disposition`, `governance-adoption`,
`human-acceptance`, or `qualification`. `subject` contains the required Warrant
identity and contract digest; result digest is required for result-related records
and absent for permission/governance records if no result exists. `actor.kind`
is `human`, `agent`, or `service`. `policy_ref` is null or a record ID; refs are
unique record IDs resolvable within supplied input or explicitly unresolved.
Missing required evidence/basis blocks the dependent judgment. Unsupported kinds,
unknown fields, duplicate IDs, wrong subject digests, and false actor-kind claims
against trusted identity facts are rejected.

`payload` is a typed JSON object, by kind:

- Claim: `statement` and `scope` strings; it is never a passing observation.
- Observation: `check_id`, `check_digest`, `execution` (`completed`,`unavailable`,
  `error`), `verdict` (`PASS`,`FAIL`,`UNKNOWN`), `artifact_refs`. Only completed
  observation can PASS/FAIL; unavailable/error has UNKNOWN with a reason.
- Inference/judgment: `statement`, `scope`, and `limitations`; supporting facts
  stay in evidence_refs. Judgment additionally names `decision` string.
- Verification: `performer_id`, `candidate_digest`, `obligations` array of
  `{id,scope,disposition,evidence_refs}`, `isolation_refs`. Disposition is
  `established`, `not-established`, or `refuted`. Verifier differs from performer;
  isolated context/workspace evidence must be present for the baseline.
- Permission: `acts` array, `constraints` object, `effective_policy_digest`.
  Permitted act names are `execute`, `amend-contract`, `adopt-sas`, `merge`,
  `deploy`, `edit-policy`. Delegated acts require trusted human-approved policy.
- Policy-disposition: `decision` string and `limitations`; cannot be human acceptance.
- Governance-adoption: `document_digest`, `decision`, `delegated` boolean. Policy
  edits affecting effective automation authority still require a human.
- Human-acceptance: `decision:"accept"`, `reviewed` array containing `outcome`,
  `verification`, `risks`, and `meaning` string; actor kind must be human.
- Qualification: `profile`, `scope`, `verification_ref`, `acceptance_ref`,
  `limitations`, `history_claims`, and `criteria`; exact subjects and all SAS §12 evidence are required.
  Criteria is an array of `{id,disposition,evidence_refs}`, with IDs `scope-permission`,
  `obligations`, `protected-expectations`, `independence`, `passing-checks`,
  `adequacy`, `human-review`, and `preservation`, exactly once each. Dispositions
  use the verification vocabulary. All must be established by trusted admissible
  records for eligibility. A caller supplies the exact normalized contract payload
  to assurance evaluation; a compiler cannot establish coverage from an agent
  selecting which expectations to mention. Repositories may supply additional
  criteria; omission of a required strengthened criterion blocks eligibility.

Provenance authenticity is `unverified` or `externally-verified`. The latter
requires trusted verifier evidence supplied out of band by the caller; writing
that string does not prove it. An offline content-only check returns unverified
for authenticity unless a configured trust verifier establishes it. A structurally
valid example human record SHALL NOT issue a real mark. Full signing/attestation
transports remain legacy or future adapters; existing signed bytes are preserved.
The library returns `eligible`, `ineligible`, or `unknown` for assurance evaluation
with per-condition reasons; only trusted established inputs can yield eligible.

<!-- ow:unit digests binding -->
## F9. Digests, compatibility, and version negotiation

Raw SHA-256 is over exact file bytes. Structured digests hash RFC 8785 JSON of
`{"digest_domain":<domain>,"payload":<object>}` using SHA-256, formatted
`sha256:<64 lowercase hex>`. RC.2 domains are `oh.war/basis/1.0.0-rc.2`,
`oh.war/contract/1.0.0-rc.2`, and `oh.war/package/1.0.0-rc.2`.
A contract payload has `warrant_source_digest`, `sources` (F5 lock entries for
required sources sorted by path), `constraints` (bound unit references), and
`expectations` (ordered `{id, scope, check_digest}` objects). Expectation IDs are
unique. The caller supplies this normalized expectation list from the reviewed
fixtures/check definitions; the compiler does not infer executable assertions
from prose. The list is part of what approval binds. A qualifying verification
must cover every listed expectation on the same candidate, with observations of
the same check digest. Missing contract/expectation information yields UNKNOWN,
not an empty list of automatically satisfied obligations. The payload excludes
mutable progress, observed results, and actor claims. Approval authenticates this
exact payload through a workflow, not through source state.

The `oh.war/document/1.0.0-rc.2` adapter SHALL NOT be selected merely because a
legacy file also contains Markdown. Legacy atoms use their existing parser and
retain their schema/digest domains. Import explicitly declares source dialect and
adapter version, preserves the original files and JSON, and emits a mapping plus
unsupported-semantics diagnostics. Migration is a new output operation, never an
in-place rewrite of authorized sources or signed payloads. Unsupported editions
fail by name. Stable 1.0 may adopt an unchanged contract or a revised one only
with explicit version and migration evidence; RC.2 is not already Stable.

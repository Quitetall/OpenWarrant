---
schema: oh.war/atom/v1
adr_uuid: 01a0f547-44a2-7d27-9d77-43d36f651cd1
local_alias: OW-ADR-0030
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
---

# ADR OW-0030: Bind a verdict to the subject that was reviewed

## Status and scope

Proposed experimental decision for [ticket t-b9610](../../tickets/t-b9610/atoms/10-intent.md). The branch is unfinished. This proposal grants no authority, changes no signed historical record, and does not adopt a new assurance baseline. Read the [implementation notes](../../design/reviewed-subject-binding-wip.md) for observed checks and remaining work.

## Problem

A public CLI reproduction ingested an independent response, changed the requirement, and still reported verification current. It also accepted the old response again. A historical observation cannot establish a different current subject.

Separate reproductions showed packet generation overwriting different retained bytes and following a packet filename link to overwrite a file outside the repository. Retained evidence must remain distinguishable from a newly generated projection.

## Reviewed subject

The request carries `reviewed_subject`. A response echoes the exact object it reviewed. The draft captures:

| Field | Binding |
|---|---|
| `contract_digest` | Existing compiled contract digest. |
| `context_sources` | Exact pinned SAS source path and ordinary SHA-256 of its retained bytes. |
| `artifacts` | Declared delivered paths and ordinary SHA-256 of actual bytes. |
| `gate_definitions` | Each cited gate key and the digest of its definition bytes. |
| `fixtures` | Declared fixture paths and actual byte digests. |
| `gate_evidence` | Recorded run, receipt, stdout, stderr and raw evidence paths and byte digests. |
| `gate_inputs` | Selected input paths and actual byte digests. |
| `gate_links` | Selected link paths and exact target text. |

Maps are sorted. Missing artifacts remain explicit and cannot satisfy existence. Malformed deliverable declarations cannot become an empty artifact set. Duplicate definitions of a cited gate cannot produce one ambiguous binding. Malformed run or receipt data refuses capture instead of becoming an absent observation. Binding a receipt does not establish its admissibility or sufficiency.

Cited gates with nonempty `inputs` use the existing glob and exclusion rules. Missing or empty input lists, missing gate definitions, and no cited gates use the existing conservative tree scope. Explicit selections are unioned with that scope: tree bookkeeping exclusions cannot remove a declared input.

Internal links bind both target identity and selected file/subtree dependencies. A link differs from a regular file holding the same bytes. Candidate Git link blobs remain private data, rather than filesystem links. Candidate Git queries and the batch blob reader disable replacement objects and lazy fetching; an unavailable object remains UNKNOWN instead of reading a substituted commit or fetching new data. External, dangling, chained, cyclic and unsupported targets are unavailable observations, reported as UNKNOWN.

Input bindings identify the reviewed workspace. They do not grant permission to publish every source as blind context. Performer rationale and historical instructions remain excluded from reviewer context. Source selection and portable context closure are separate requirements.

## Transport and qualification

The experimental request/response transport is v2. A v2 response requires the reviewed subject and exact packet references before any verdict write. V1 responses remain historical observations, including those containing optional newer fields. An unsupported future response version reports UNKNOWN; malformed current v2 data is refused.

Existing v1 records remain byte preserving on read. New observations store the original verification payload inside an explicit v2 envelope; an older bare-record reader must refuse that envelope. The verification-recorded journal event binds the record digest, actor, Warrant identity, obligation, reviewed subject, packet references and explicit v2 protocol. Protocol labels and reviewed bindings are never inferred from timestamps or optional fields.

Ingestion preflights the whole response: current subject, declared obligation identifiers, packet identity, performer and obligation coverage. Qualification uses that same rule for preparation, resolution, progress and the assurance mark. Legacy or stale observations remain on disk and supply no current qualification. Known independence failures retain their inadmissibility reason.

Candidate acceptance compares against exact Git-tree data using the same subject/compiler routines. Its private snapshot does not run candidate code or activate candidate-selected authority. Unsupported selected nodes remain visible as unavailable observations.

V1-only response readers refuse v2 envelopes. This does not protect stored records from older readers that ignore the new journal binding. A direct synthetic CLI probe of the installed alpha.2 release reproduced stale observations counted as established, including passing the obligation and independence requirements in a resolution dry run. That reader also ignores `project.requires_war`; a minimum-version setting alone does not protect it. The current reader rejects those stale observations. Adoption therefore needs a record-level storage boundary that old bare-record readers cannot misread, alongside the frozen v1 reader and preserved history. Define the new envelope and schema-pack adoption before qualified release; do not silently alter the frozen v1 payload or claim a version setting covers readers that ignore it. The portable probe is `conformance/fixtures/verifier/probe-stored-reader.py`; its results apply to the exact recorded executable digests, not every old tool.

## Proposed stored-record boundary

Newly ingested observations use a distinct `oh.war/verification/v2` envelope.
The frozen v1 `Verification` payload is nested under `verification`, without
renaming its fields. The root must not repeat v1's `obligation`, `disposition`,
`performer`, `evidence` or `verifier`: the observed older reader requires these
at the root, so a nested payload causes a parse refusal rather than a verdict.
The envelope carries the response protocol, reviewed subject and exact packet
references. A v1 response can be retained in this envelope as unbound history;
it does not gain qualification by being wrapped.

The current reader continues to read existing bare v1 records without rewriting
them. Current qualification requires a v2 envelope whose bindings agree with
the journal event, current subject and retained packet bytes. Candidate
re-verification uses the same record decoder and binding checks. Unknown future
formats must not fall back to a permissive v1 decoder.

This is an experimental wire-format proposal, not adoption of a schema pack.
The pack must publish v1 and v2 beside each other, move its format version, and
support explicit adoption without reinterpreting old contract digests. Until
that path and history retention are proved, this branch remains unreleasable.
The regression probe's `--require-old-refusal` checks both fresh current-reader
qualification and old-reader refusal, so rejecting all reviews cannot pass it.

Before replacing an active verification, retain its exact bytes under
`verifications/history/<raw-sha256>.toml`. The existing journal digest refers
to these bytes; retaining them introduces no new digest domain or authority.
History is outside the active-record enumeration. Immediate
`history/<64-lowercase-hex>.toml` paths are verification bookkeeping for the
implicit tree scope, like active verdicts; archiving a review must not change
its own default source binding. Declared inputs still bind these files, and
other names or nested files in that directory stay in the conservative tree.
A no-overwrite publication
must reuse identical bytes and refuse collisions, links or unavailable files.
If retention fails, do not replace the active record. Unchanged replays must
remain byte preserving. A changed subject needs distinct bound bytes and a
fresh current observation; an archive alone is not qualification.
`--require-history-retention` reproduces lost prior bytes at the public CLI
seam and checks fresh re-review plus exact archival. This control is still red
before the retention implementation.

## Portable packet binding

Each `reviewed_packets` entry carries a retained repository-relative path and the full existing VerificationBundle digest. The path uses the existing shortened digest filename; qualification checks the full digest. Packet/request schema, exact subject, performer and covered obligations must match. Qualification reads the retained packet; it does not rebuild a different packet after prior verdicts changed.

Contract and packet digests retain their existing Rust canonicalizers and digest domains. No replacement canonicalizer, Python digest implementation or model-created digest is introduced. `verify --bundle --json` returns a machine-readable packet index; configured verifier calls receive its Rust-computed references as transport metadata.

Packets carry exact required pinned-SAS, gate-definition, fixture and recorded-evidence bytes. UTF-8 stays readable; binary fixtures stay lossless byte arrays. Every obligation packet retains those fixed inputs. Budget pressure cannot silently remove or excerpt them. Assembly compares them against the captured subject and refuses required sources that change or disappear during capture. The draft locates the captured SAS version/digest in retained revision records, uses descriptor-safe current reads, and recovers matching historical Git bytes locally when needed. Candidate snapshots cannot borrow mutable checkout history. The historical-source extension freezes the candidate commit and reads regular source blobs only from that commit’s retained ancestors, with replacement objects and lazy fetching disabled. Matching content must have the exact pinned SHA-256; a newer source file or unrelated branch cannot substitute. An unavailable pinned source is UNKNOWN. This extension has a reproduced CLI failure and is awaiting its green qualification run. Packet qualification also checks the full carried source against its subject digest. Request and packet assembly reuse one loaded Warrant. This SAS slice is under test; complete transitive governing-context closure remains open.

## Retained packet storage

The draft storage implementation reuses identical retained bytes and refuses different bytes at the same filename. New packets are staged and synced, then published without replacing an existing name. A competing publisher must supply identical bytes; it cannot truncate the winning packet. Directory-relative file handles refuse link traversal, and packet reads inspect regular files through the opened descriptor.

Missing, unreadable or unsupported retained files report UNKNOWN. Observed identity/content mismatches remain refusals. History remains intact; regeneration is not permission to repair or overwrite retained evidence.

These controls do not sandbox a same-uid writer, prove verifier private-copy custody, or complete the source-capture race audit. Those claims require separate evidence.

## Configured verifier

The wrapper reads a private copy of the supplied packet and echoes its request subject. It must not recapture a newer subject after review, infer a missing legacy binding, or ask a model to invent one. Rust transport metadata binds packet references. Complete copy/custody and harness-isolation qualification remains open.

## Adoption requirements

Before release, prove complete context/evidence closure, stored-record reader compatibility, large-packet budget behavior, unsafe-file and race controls, and the full gate. Independent verification and secure human acceptance remain separate acts. This draft and its local checks satisfy neither act.

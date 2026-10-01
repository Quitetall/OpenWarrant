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

Existing core verification record bytes stay unchanged. The verification-recorded journal event binds the record digest, actor, Warrant identity, obligation, reviewed subject, packet references and explicit v2 protocol. Protocol labels and reviewed bindings are never inferred from timestamps or optional fields.

Ingestion preflights the whole response: current subject, declared obligation identifiers, packet identity, performer and obligation coverage. Qualification uses that same rule for preparation, resolution, progress and the assurance mark. Legacy or stale observations remain on disk and supply no current qualification. Known independence failures retain their inadmissibility reason.

Candidate acceptance compares against exact Git-tree data using the same subject/compiler routines. Its private snapshot does not run candidate code or activate candidate-selected authority. Unsupported selected nodes remain visible as unavailable observations.

V1-only response readers refuse v2 envelopes. This does not protect stored records from older readers that ignore the new journal binding. A distinct reader rollout/version guard remains required before release.

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

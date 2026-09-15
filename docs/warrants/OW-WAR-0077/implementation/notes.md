# OW-WAR-0077 implementation

Base: 8f688e1e531a0a9bc7a5711a78c4583d8cc09f3e. Phase 1 SDK implemented and tested; Phase 2 pending. Unverified.

Public seam: RC.3 format F3/F5 source descriptors and bound references, checked
against explicitly supplied bytes. Source acquisition, path confinement at I/O,
source-change detection during capture, dependency selection and authority checks
remain provider/caller responsibilities. The SDK must not hide these operations.

Phase 1: typed descriptors/identities/references, bounded codecs, source integrity
and exact declared range checks, direct file-backed conformance driver.
Phase 2: explicit provider response integration; no LAMU support claim without
its actual implementation passing interoperability cases. Initial local LAMU
source inspection found no matching OpenWarrant capture/resolution endpoint.

Requested future workflow: a checklist and start-work actions in the web viewer.
This remains outside 0077 and the read-only progress viewer; not implemented here.

## Public SDK boundary

`openwarrant_core::document::source` exposes `Holder`, `SourceMetadata`,
`DocumentIdentity`, `SourceDescriptor`, `BoundReference`, `SourceLimits` and
`CheckedSources`. Wire structs are unchecked claims until the public checks run.

- `describe_source`: derive identity and raw SHA-256 from explicit bytes and an
  explicit dialect (or None for opaque bytes); no path lookup or source capture.
- `decode_source_descriptor` / `encode_source_descriptor`: bounded F5 record codec.
- `decode_bound_reference` / `encode_bound_reference`: bounded F3 reference codec.
- `check_sources`: compare declared source facts with supplied blobs; refuse missing,
  changed, ambiguous or invalid sources. One hash/parse per unique digest.
- `CheckedSources::check_reference`: return an exact borrowed whole-file/unit slice
  only when digest, unit ID and declared byte range agree. Indexed lookup.
- `CheckedSources::canonical_lock`: RFC8785 path-ordered descriptor array, preserving
  caller metadata. No new digest domain or signed subject is minted.

`cargo run -p openwarrant-core --example sdk_probe -- --scope 77 --fixtures conformance/sdk`
exercises 19 literal cases. Dedicated user CLI parity belongs to OW-WAR-0087.

Limits are positive and inclusive: 8MiB per blob, 256MiB total supplied blob plus
encoded descriptor bytes, 4096 descriptors/blobs, 65536 distinct parsed units,
1MiB per descriptor/source metadata, 64MiB encoded output. Description counts its
single blob and descriptor; codecs bound their record input/output; source counts
and aggregate unique units apply to source sets. Every supplied blob counts toward
the byte budget, including unused blobs. There is no truncation or hidden I/O.

All fields remain claims. In particular, holder locators and established authority
labels are not authenticated. A correct hash never proves authority or access.
Unsupported required extension semantics remain unevaluated; validity/integrity
checks cannot claim full context, workflow readiness or provider qualification.

## Current observations

11 public tests and 19 conformance cases pass. Independent Spec/Standards reviews
pass after the total-byte repair and repeated-work cleanup. Aggregate gate at
593fbc287ba3287579b43604d0de1406ab1c1505 passed all 14 steps: 762 Rust tests
and 308 planted refusal cases. Publication remains pending. Git whitespace warnings on exact CRLF/range
fixtures reflect intentionally retained bytes, not editable prose formatting.

Phase2 remains pending under the existing mixed-phase Warrant. Its actual-provider
acceptance cases are documented in conformance/integration/source/README.md.

Local LAMU commit review returned PASS WITH NITS. Findings were checked: the
counting writer refuses oversized output before allocation; literal test slices
independently check parser ranges; authority labels are restricted claims, not
authentication. No confirmed defect remained. Review output is retained in
`../evidence/source-sdk/commit-review.log`.

## Real-provider integration candidate

The optional LAMU `lamu-openwarrant` crate implements explicit local source capture
and F3 resolution, using the published SDK at 8f048eee. The shared contract is
`docs/integrations/lamu-source-provider.md`. The SDK still performs no filesystem I/O.

Five filesystem tests pass through the cross-repository driver, including T11–T15
and an aggregate-budget regression. Independent Spec and Standards reviews found
late metadata accounting; incremental preflight/accounting fixed it and both
reviewers reproduced the repair. Immutable snapshots now cache unit indexes and
canonical locks, avoiding repeated corpus parsing during reference lookup.

Provider publication and participant merge remain pending. LAMU's existing default
Rust 1.89 formatting and Clippy checks fail in unrelated runtime code. Its new
optional provider passes focused Rust 1.97.1 tests, formatting and Clippy. No live
LAMU service or unrelated local changes were replaced. Sequential file rechecks
cannot establish atomic cross-file snapshots; this limit is explicit in the profile.

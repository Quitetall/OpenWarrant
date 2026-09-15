# OpenWarrant and LAMU source-provider contract

Revision: 1, implementation draft. Canonical shared contract: this file in the
OpenWarrant repository. Participant receipts bind its raw SHA-256 and their exact
repository revisions. LAMU keeps a pointer and digest, not a separately edited copy.
No official shared Warrant identifier, human signature or assurance mark is allocated.

## Participants and scope

- OpenWarrant base: 8f048eee8c5679660e20537f82ddf1b923b21a3f.
  OW-WAR-0077 owns SDK source types and integration conformance.
- LAMU base: a63d20042843fb921bd9262566d47a640f1c52cf.
  LAMU owns filesystem acquisition and reference resolution in an optional
  `lamu-openwarrant` crate. It consumes the OpenWarrant SDK; it does not copy the
  document parser or canonical source types.
- OpenWarrant writes: this shared contract, integration fixtures/driver,
  OW-WAR-0077 implementation/evidence records, and necessary SDK source helpers.
- LAMU writes: `lamu-rs/lamu-openwarrant/`, workspace membership/lockfile, focused CI, dependency-source allowlist,
  dependency ADR and a participant pointer/receipt. Existing runtime, retrieval databases and live
  services are outside this change.

The owner's instruction to complete 0077 permits this bounded unverified work.
No pre-work signature is required. Independent testing/review is required before
reporting integration complete. Qualification and secure human acceptance remain
separate; this contract cannot grant filesystem access beyond the invoking caller.

## Source capture interface

The provider receives an explicit root directory capability, declared source paths,
explicit document dialect or opaque interpretation, source metadata, requested
references and positive resource limits. No directory discovery, network retrieval,
model call or ambient source inference occurs. Source paths follow F3; all symlinks
are refused. Open files relative to the root capability without following symlinks;
check regular-file identity and size before reading. Bound reads incrementally.

The provider derives each descriptor from actual bytes using the SDK and retains
those original bytes. `begin_capture` reads sources; `PendingCapture::finish` reopens each path and
checks file identity, size, mode, ownership, link count, change/modification times
and exact content. A detected change refuses the whole capture. These checks are
sequential, not an atomic cross-file snapshot: a file can change after its final
check. Callers needing an atomic repository revision must supply an immutable
checkout or externally exclude writes. Subsequent use binds only captured bytes.

Reference resolution uses explicit origin and F3 target syntax over the captured
source set. Same-source targets, named units, parsed whole-source targets and raw
whole-file targets retain exact raw-byte ranges. Opaque named-unit targets refuse.
No dependency selection or completeness of a semantic package is claimed here.

Results include sorted SDK descriptors, original blobs, bound references, provider
identity/build, profile identifier, exact shared-contract digest and limitations.
Canonical locks use the SDK's existing RFC8785 representation. Enumeration order
must not affect successful source locks or reference outputs. Source identity,
byte length, digest and range claims are checked by the consuming SDK.

Diagnostics distinguish `target-invalid`, `target-missing`, `source-ambiguous`,
SDK `source-invalid`/`digest-mismatch`, `source-changed`, `source-forbidden`,
`resource-limit` and `source-unavailable`. During final recheck, any read failure
is `source-changed`, with its underlying diagnostic code in the message. A failed operation publishes no replacement
snapshot. An existing caller-held result remains unchanged.

## Integration and exit

1. Implement source capture/resolution in LAMU using the SDK's exact published Git
   revision. Keep this crate outside LAMU's default runtime dependency path.
2. Run T11–T15 against that implementation, with Warrant, ADR and opaque bytes;
   heading rename; missing target/conflicting identity/digest tamper; traversal,
   absolute path, symlink and controlled capture-time mutation; reordered inputs.
3. Record provider/SDK/contract/build identities, input hashes, exact outputs and
   positive/refusal observations. Fake-provider tests cannot satisfy this stage.
4. Independently review both participant changes and reproduce integration cases.
5. Merge compatible participant changes, then integration evidence. Git merges are
   not atomic across repositories. Failed checks leave previous snapshots and all
   historical records intact; revert unaccepted implementation if necessary.

## Profile `lamu.openwarrant.local-source/1`

This is an in-process Rust API profile, not an HTTP, MCP or database endpoint.
The optional `lamu-openwarrant` crate requires Rust 1.97.1, matching its pinned SDK;
it is excluded from LAMU default members and runtime dependencies. Existing LAMU
components retain their own Rust 1.89 toolchain. Linux is exercised here; the
Unix implementation is intended for macOS but macOS runtime proof remains absent.

Request: `begin_capture(&File, &[SourceRequest], SourceLimits)`. Each source has
`path`, `dialect: Option<Dialect>`, `holder: Holder`, `metadata: SourceMetadata`.
`None` means opaque bytes. Root must be an open directory; possession of that
handle and host filesystem permissions form the access basis. Metadata labels
and holder locators confer no authority. The caller supplies trusted access
policy before invoking the provider; this profile has no policy engine.

Response: pending capture, then `finish() -> Result<Snapshot, Diagnostic>`.
Failed calls return no Snapshot and do not mutate a prior Snapshot. Read-only
accessors expose SDK descriptors and digest-indexed original blobs. `resolve`
takes explicit captured origin and target strings and returns an SDK
`BoundReference`; raw paths select whole blobs, `#*` requires a parsed document,
and named selectors require exact stable unit IDs. `canonical_lock` delegates to
the SDK's bounded codec. All paths are root-relative. No serialized provider
response or network transport is part of version 1.

`profile()` declares provider name/version, profile ID, SDK revision and shared
contract SHA-256. A test/build receipt additionally binds the exact provider Git
revision or changed-file hashes, Cargo.lock and Rust toolchain. Profile declarations
are not authenticated attestations. Sorting affects descriptors/lock bytes;
individual resolution results depend on explicit targets, not enumeration order.

SourceLimits defaults are the SDK defaults. Capture additionally counts bytes per
requested path plus encoded descriptor metadata, including aliases, against
total_bytes to bound actual reads and retained records;
SDK source-set checking also counts encoded descriptors and unique blobs. Reads
stop before exceeding source_bytes/total_bytes. Borrowed request labels are counted
before cloning. Unit spans are indexed during capture under a cumulative unit
quota (aliases count per path); successful snapshots cache their canonical lock.
No truncation is permitted. This source-only integration does
not advertise full context compilation, execution readiness, LAMU database storage,
condition evaluation (0078), dependency closure or an offline semantic package.

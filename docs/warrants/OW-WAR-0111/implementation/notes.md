# OW111 progress — experimental codec

Pure library codec added in compiler/preservation.rs under explicitly draft
transport identity. Existing PortableExport, legacy digests and pinned journal.rs
unchanged. Four focused test groups pass; compiler package all-target Clippy with
-D warnings passes on Rust 1.97.1. No new dependencies.

Tests preserve embedded binary bytes (including CRLF/NUL), opaque extensions and
canonical round-trip identity; exercise actual external evidence resolver success,
missing/tampered/unavailable refusal, category omissions, path collisions,
unsupported versions, byte/count limits, and duplicate JSON input keys.

Limits and safety bound: codec takes already acquired bytes. Resolver must bound
its I/O; returned aggregate content is checked. Codec checks declared coverage
structure, not truth of source-category classification. Assembly must prove that.
No filesystem materialization, authority activation, real corpus archive, historical
record import, CLI replacement or KF round-trip exists yet. Old false CLI success
is still a known open defect; codec does not claim it repaired. Format remains a
proposal, not stable standard or human-accepted amendment.

Next: assemble actual retained categories, implement isolated import and source-
detached re-export, then replace false CLI proof and test KF interoperability.

## Inert filesystem import and detached re-export

`war archive import INPUT DESTINATION [--evidence DIRECTORY]` now decodes the
experimental archive, reconnects actual bytes, and creates a new private inert
bundle. Descriptor-relative no-follow opens and exclusive creation protect paths.
Completion marker is written last; partial failures retain diagnostic files and
cannot re-export as complete. Existing destinations and output files refuse.

`war archive reexport DIRECTORY OUTPUT` re-reads every imported record and checks
its digest, including originally embedded records. Original source archive and
external evidence directory can be removed before re-export; resulting canonical
archive bytes remain identical. No repository configuration or authority is
activated. JSON mode returns the common report envelope with authority_activated
false. Linux/macOS implementation only; no Windows capability claim.

Four actual CLI integration tests pass: detached embedded/history-label fixture,
external evidence with source deletion, overwrite/tampering/symlink refusal, and
machine envelope. Strict CLI all-target Clippy passes. History labels here are
synthetic bytes, not real signed supersession/dispute/annulment qualification.

Still missing: actual repository category assembly, IR reconstruction from imported
sources, full historical selection, legacy export claim repair, accepted format
decision, and real empty-KF-instance round trip. OW111 remains incomplete.

## Actual current-source capture and reconstruction

`war archive export ALIAS OUTPUT` captures the current Warrant tree through
bounded no-follow reads, compares loader inputs with captured bytes, embeds a
versioned Basis descriptor and canonical IR, and records unresolved categories
as unavailable. It returns complete:false, never a full-preservation claim.
`war archive inspect INPUT` reconstructs IR from embedded manifest/atom/scope/SAS
pin inputs and compares exact canonical bytes and Warrant subject. It requires no
source repository. Import/re-export also check reconstruction when that descriptor
is present. Historical contract source lookup is not implemented by this capture.

Five CLI integration tests pass, including a newly initialized actual Warrant,
source docs moved away, altered IR with a recomputed record digest refusing, and
wrong-subject refusal. Strict all-target Clippy passes. Real OW-WAR-0001 current
snapshot and export/inspect observations retained here. That snapshot deliberately
marks historical/provider categories unavailable and cannot claim full import.

Next remains full historical/category assembly, external provider records, legacy
false-proof replacement, formal format decision and actual KF preservation test.

## Legacy false-success repair

Legacy `war export --round-trip --reconnect` now refuses explicitly: a caller
flag is not observed import. No legacy envelope/digest or pinned core record
changed. Obsolete global claims that no Warrant has resolved or nothing executed
are replaced with the actual limitation: this exporter does not collect those
records. Six CLI integration tests pass, including the false-success regression.

The old battery's false positive becomes a named refusal. A separate positive
control imports and re-exports exact experimental transport fixture bytes; its
actual shell code was run successfully in isolation. This positive is bounded
transport evidence, not full §68.3 KF qualification. Battery grows by one control;
full new-head gate remains pending. Strict CLI all-target Clippy passed before
comment/reason wording updates; no new dependencies.

## Bounded Git history and delivery-record provenance

`archive export --history` now includes raw commit objects and regular Warrant
file blobs for history reachable from pinned HEAD, plus a canonical history index.
No checkout, signing, remote fetch or imported authority. Bounded records/bytes,
256 relevant commits and bounded Git subprocess reads; shallow/unavailable history
refuses. Other refs are explicitly excluded in the index and completeness remains
unavailable. This is not an assertion that all institutional history is present.
Seven CLI tests pass, including exact historical fixture bytes and source-detached
inspection; shallow history refuses before output creation. Strict Clippy passed.

The first full gate at e278f3c failed three steps: existing OW30 D-001 and OW63
D-003 delivery hashes still named prior bytes, causing corpus/projection and
positive-control failures. Failure log retained. `war correct` confirmed both
Warrants are unresolved and directs performer-record regeneration rather than a
human correction act. Original delivery records AND original artifact bytes are
preserved under legacy-before-repair/ in their original relative structure, with
hash/index and the exact tool observations. Current entries now name observed new
hashes, performer agent://codex and OW111's unverified repair attempt. Existing
verification/judgment/resolution records are unchanged. This does not resolve
OW30/OW63 or claim their old evidence verifies new code.

Recompiled projections after the record update: 966 pass, 88 warnings, zero errors
and zero unknown. Full new-head gate still required after history implementation.

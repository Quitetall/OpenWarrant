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

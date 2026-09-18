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

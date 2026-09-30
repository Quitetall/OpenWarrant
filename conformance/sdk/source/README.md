# Supplied-source SDK conformance

Run `cargo run -p openwarrant-core --example sdk_probe -- --scope 77 --fixtures conformance/sdk`.
The driver reads explicit fixture files and calls the public SDK. It is not a
source-capture provider and does not follow document pointers or inspect live paths.

These fixtures cover source descriptors, original-byte digests, exact unit ranges,
canonical path-ordered locks, changed content, conflicting identity claims, unsafe
path syntax, opaque inputs, CRLF and UTF-8. Literal expected locks use the F3/F5
record shapes and RFC 8785; offsets index the supplied raw bytes. CRLF fixtures
are protected from Git normalization by local attributes. Cases use reserved
example identities, never real actor or authority allocations.

Raw source bytes remain separate from wire descriptors. Holder and metadata
fields are retained claims, including an `established` label. Passing these checks
establishes neither access permission, authenticated provenance nor semantic
context completeness. Missing required extensions remain a compiler concern;
structural source checks do not advertise extension implementation.

The public Rust tests additionally cover decoder duplicate/unknown fields, wrong
digest domains, invalid ranges and positive inclusive limits. SDK checks cannot
observe a symlink or source change during acquisition: those T14 scenarios belong
to the actual provider integration in `conformance/integration/source/`.

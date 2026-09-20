# OW-WAR-0081 implementation

In progress, unverified. SDK integrity slice based on 69a67baf.

The packet module exposes strict F7 wire types, bounded canonical decoding and
encoding, F9 structured digests, exact source/range checks and whole-package file,
root and basis integrity. Package validation consumes a regular-file map and
moves blob buffers; filesystem callers must enforce no symlinks/special files.
Source checks allow omitted opaque candidates while requiring every parsed
routing source. The existing strict source API remains strict.

Seven public tests and seven real-file driver cases pass. Full core tests and
Clippy with denied warnings pass on Rust 1.97.1. Run:

    cargo +1.97.1 test -p openwarrant-core --test document_packet
    cargo +1.97.1 run -p openwarrant-core --example sdk_probe -- --scope 81 --fixtures conformance/sdk

Independent Spec and Standards review found incomplete F5 basis acceptance,
missing typed-packet size enforcement, noncanonical float byte counting, and
unbounded recursion in structured digests. Red/green public regressions prove
repairs; independent scratch rechecks passed, including 20000 nested arrays
returning resource-limit without process abort. Caller-owned values stay owned
by callers; SDK validation does not control their later drop behavior.

An intentionally self-consistent but semantically incomplete package passes
integrity and explicitly reports semantic coverage unestablished. Provider-owned
selection reconstruction must reject it before claiming complete context.
No readiness, authority or assurance follows from these checks.

Provider export, offline semantic checking, atomic output and full integration
T32-T37 remain pending. This SDK slice does not complete OW-WAR-0081.

## Actual LAMU provider integration

Portable task packages now build, read and verify offline through the actual LAMU
provider. Semantic reconstruction rejects a resealed incomplete context package
that passes SDK integrity. Optional opaque descriptors may remain unsupplied.
Fresh-directory publication rejects overwrite, traversal and symlinks.
Provider tests: 23 passed, Clippy passed; independent specification and standards
reviews passed after fixing pre-allocation basis checks and package limit alignment.

Commit reviewer nits were checked: canonical output has an explicit post-JCS byte
check; parser receives the remaining unit quota before subtraction. Both alleged
defects were false positives. The reported readiness formatting line was not in
the cited package implementation.

No human assurance or Phase 1/2 exit is claimed. Record/readiness integration and
participant merges remain prerequisites for the complete downstream workflow.

# RFC 8785 conformance vectors

Vendored verbatim from
[`cyberphone/json-canonicalization`](https://github.com/cyberphone/json-canonicalization),
`testdata/input/` and `testdata/output/`, retrieved 2026-08-19.

Six input/output pairs: `arrays`, `french`, `structures`, `unicode`, `values`,
`weird`.

## Why these are committed rather than fetched

A conformance suite that downloads its expectations at test time is not a
conformance suite — it is a network call that usually returns the same answer.
It fails when the network does, and it silently changes meaning when upstream
edits a file. These bytes are the expectations this implementation is held to,
so they live in the repository under version control like any other contract.

## Why they are external expectations

`openwarrant-compiler` is tested against **these** bytes, never against a
snapshot of its own output. A self-derived snapshot asserts only that the
canonicalizer is deterministic; it would pass unchanged if the key sort used the
wrong collation or the number formatter used Rust's shortest round-trip instead
of ECMAScript's, and every digest built on it would be wrong in a way no test
could see.

The published outputs carry a trailing newline. Canonical JSON does not, so the
test trims it — the file is a text file, the canonical form is the bytes inside
it.

## What is NOT covered

The upstream `es6-numbers` file — millions of generated float cases — is not
vendored; it is far too large to hold in a repository this size. The ES6 number
boundary cases are instead pinned directly in
`crates/openwarrant-compiler/src/canonical.rs`: the values where ECMAScript
number serialization diverges from a naive shortest-round-trip formatter
(`1e21`, `1e-7`, `5e-324`, `-0`, the f64 extremes). That is narrower coverage
than the full file and is recorded here as such rather than left to look
complete.

# OW-WAR-0086 implementation status

The typed SDK compiler boundary and actual LAMU adapter are implemented. A second
minimal consumer uses only OpenWarrant SDK, reads the generated offline package,
and reports integrity separately from provider semantic evidence. All four roles
retain blocked readiness until record evaluation is connected. No human assurance
or Phase 2 completion is claimed.

The attached receipt reruns source, condition, selection, projection, package and
budget provider cases, actual adapter tests, repeatable generation, independent
SDK consumption, changed-byte refusal and duplicate-file refusal. Source and
provider revisions, contract and input hashes are retained.

Independent specification and standards reviews passed after repairs to task
identity validation, deeply nested borrowed records, and provider pre-allocation
checks. Twenty-eight provider tests and Clippy passed. Reviewer claims of a double
`info()` call were checked against source: `compile_with` calls it once. Distinct
depth and serialized-byte checks are deliberate, not duplicate permission checks.

## Remaining prerequisite work

Full OW-WAR-0086 remains blocked on OW-WAR-0085 Phase 1 exit. The incomplete SDK
record/readiness slice (0083), legacy/successor slice (0084), CLI parity (0087), and
consumer/skill exit checks must pass before Phase 2 exit can be established.
The actual context compilation/package integration is proven within the attached
scope. Participant changes still require compatible merges after CI; no legacy
signature or disposition was changed to pretend those prerequisites were met.

Next implementation: 0083 records and readiness, then 0084 legacy/successor handling,
0087 CLI parity, 0085 Phase 1 exit, and remaining 0086 record/legacy exchange checks.

## LAMU PR review and Windows isolation repair

PR2 head 1f3b422e introduced a Windows checkout regression: Cargo resolved the
optional Git SDK while checking ordinary LAMU, and preserved archive filenames
contain colons. The provider now has a separate Cargo workspace and lockfile,
excluded from main LAMU resolution. Both CI layouts run its explicit manifest and
separate dependency audit. Original archive bytes and paths remain unchanged.

Locked offline Rust 1.89 dependency resolution for the Windows target succeeds
with zero OpenWarrant packages. The isolated provider passes 28 tests, Clippy and
cargo-deny. Independent review passed. Native Windows compilation and fresh remote
CI are not established by this Linux metadata observation. The prior main Linux
CI failure was runner communication loss; no code test failure was observed.

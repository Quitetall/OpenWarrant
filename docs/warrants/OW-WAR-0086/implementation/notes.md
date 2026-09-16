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

## Merged provider and current SDK exchange

LAMU PR #8 is merged at `5f4a9054c5055a92dcb35c1804c93e62d4398c96`.
All ten PR checks passed, including Linux and native macOS provider checks. The
post-merge main run could not start because GitHub reported an account billing
limit; that is not a successful post-merge run or an observed source failure.

The real merged provider passes all six integration suites and adapter tests on
this Linux host. A second SDK consumer independently checks package integrity;
changed bytes and duplicate entries refuse. The new exchange binds records to the
actual package result, preserves unknown readiness without trust, rejects a wrong
result, preserves legacy paths/bytes and recompiles the restored task identically.
The receipt hashes tested SDK/provider build inputs before and after execution.

Phase 2 exit remains blocked by native macOS Phase 1 evidence and final full-phase
review. This does not replace historical receipts or create a human acceptance.

Independent review caught an initial use of the compile-basis digest as a contract
digest. The final exchange constructs and retains a normalized candidate contract
from actual task/source references and a bounded package-integrity expectation.
The SDK computes the separate Contract-domain digest. A direct assurance probe
checks that contract binding; substituting the Basis-domain digest produces an
unmet contract finding. No human qualification is established by either probe.

Independent specification review replayed all five contract/readiness CLI requests and passed. Standards recheck passed the source and recorded exchange. The final full Linux integration rerun passed after the contract-domain fix.

LAMU review_commit for `31d4502` returned PASS WITH NITS on the local/free model.
Its claimed readiness false-pass is incorrect: the probe refuses every state other
than `unknown`. Python dictionary comparison intentionally ignores key order.
`os.walk` does not follow directory symlinks by default; the review stated otherwise.
The input capture now explicitly refuses non-build symlink directories too, so an
unfollowed source directory cannot be silently omitted from the inventory.

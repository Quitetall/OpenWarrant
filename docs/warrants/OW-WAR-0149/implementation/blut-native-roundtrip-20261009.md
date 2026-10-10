# Real local BLUT roundtrip

Performer observation, 2026-10-09. This is a prototype integration fixture,
not independent qualification, participant acceptance or human assurance.

The local producer executed BLUT's real `connector_object` CPU stage from a
PlanSpec. It declares an object reference; it does not fetch or establish the
referenced object's availability. The receipt binds a real OpenWarrant prototype
Dispatch and its compile event from a copied conformance repository. The
fixture's sources were floating, without established Git revision anchors; the
Dispatch warnings remain in the evidence. No fixture Warrant was authorized or
resolved.

BLUT's native process verifier checked the original Ed25519 seal, expected
binding, PlanSpec, actual catalog, retained producer binary and ten native files.
The key is deliberately public test material. Only its public half was written;
this provides no operator identity or production key-custody evidence.

A separate Rust 1.97.1 consumer then used the public OpenWarrant CLI-library SDK:

1. `BlutProcessVerifier::for_recorded_dispatch` derived binding from the retained
   Dispatch and compile event, rather than a caller's receipt assertion.
2. The native verifier accepted the original bytes. Changing a signature byte in
   otherwise valid transport refused. A different expected catalog refused
   through `runtime.registry-mismatch`.
3. `runtime_capture::import` retained the bytes and emitted a matching native
   observation with `assurance_granted: false`.
4. `resolve::assess_with_runtime` matched only runtime requirement 12. Exact
   authorization and independence stayed false.
5. Temporarily removing the current native job made current assessment UNKNOWN;
   the stored historical matching observation did not clear it.

Native catalog identity was resolved by BLUT's new `registry_digest(&Registry)`
before execution. It remains BLAKE3. The local capture identity is
`sha256:21d3644a7d4824b2115b887ae6962c4b71f3ba0c78c15b8bd559e61949feb68e`.
The consumer has its own offline-resolved lockfile; this test does not replace
the locked workspace gate. Confinement, dollar cost and spend-cap enforcement
remain UNKNOWN. CLI native enrollment and operator trust protection remain open.

The job, original receipt, retained producer/verifier binaries, copied source
repository, capture, consumer source/lockfile and logs are preserved in
`~/Projects/OpenWarrant/docs/runtime-evidence/blut-native-roundtrip-20261009/`.
The native receipt also validated after relocation with unchanged job bytes.
Historical capture references retain their original locations; they are not
silently rewritten into new history. The fixture is explicitly unqualified.

OpenWarrant source: adapter `80074126`, progress checkpoint `797e5485`.
BLUT source: catalog/producer `2de3292`, documentation `eca4ec7`; PR103 remains
draft. Independent acceptance, actual operator collection, other provider
interfaces and whole-Warrant completion are not established by this fixture.

## Explicit root boundary found during the repository gate

The first gate run found that CLI `--root <empty child>` could discover its
parent's `openwarrant.toml` and operate there. A direct reproduction exited zero
while checking the parent. CLI repository selection now opens an explicit root
directly; implicit current-directory discovery keeps its existing ancestor
behavior. A regression builds a parent program and requires the empty child to
refuse, naming that child.

The initial gate also used a temporary directory inside the checkout. That made
"no Git" and "outside repository" fixtures inherit the enclosing repository and
added scratch to source inventory. Its tests/corpus failed; remaining plants
were stopped with SIGTERM after those failures. The plant trap restored all
tracked corpus bytes. This is a failed, interrupted gate, not a passing gate.
Further checks use a sibling scratch directory outside the checkout. No
checker or authored obligation was narrowed to hide these failures.

## Repeatable public integration test

The opt-in test in `crates/openwarrant-cli/tests/runtime_capture.rs` uses separately
built BLUT examples, without linking BLUT into OpenWarrant. On pinned Rust 1.97.1
it passed one native roundtrip test (`native-regression.log`): actual CPU stage,
native signature, retained Dispatch, capture and current resolution assessment.
It refused altered signature bytes, altered output bytes and a wrong catalog; a
missing current job remained UNKNOWN. The other fifteen tests were filtered out
of that selected run. The default suite does not run this external-provider test.

Build BLUT's `openwarrant_fixture_producer` and `openwarrant_verifier` examples
with its `openwarrant-receipts` feature, then set `OW_BLUT_FIXTURE_PRODUCER` and
`OW_BLUT_NATIVE_VERIFIER` to their absolute paths and run:

```sh
cargo test --locked -p openwarrant-cli --test runtime_capture \
  actual_blut_cpu_receipt -- --ignored --nocapture
```

Use a temporary directory outside any checkout. The explicit-root regression
suite also passed all four tests (`root-boundary-green.log`). Neither result
replaces the aggregate gate or independent qualification.

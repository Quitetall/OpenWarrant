# Candidate SDK receipt boundary

This is partial, unverified implementation of OW-WAR-0149. It supplies an
in-process Rust interface in `openwarrant_core::document::runtime`. It does not
publish a provider wire protocol, receipt producer, durable store, CLI import,
or resolver integration. The shared Warrant and its ticket remain open.

## Inputs and trust

The caller selects one provider interface by kind, source/build identity and
interface version. The caller supplies the actual checked, recorded
`StageDispatch`, its resolved capability policy, actual BLUT lowering registry,
confinement requirement and spend policy. This function checks required dispatch
fields. It does not authenticate the caller, verify a canonical dispatch digest,
or resolve the policy reference. An arbitrary constructed dispatch is not proof
of a recorded attempt.

The selected `ReceiptVerifier` verifies provider-owned seals and native records.
It must bind all returned facts to the same captured raw bytes and native
session/job. It owns terminal-status mapping, native execution observations,
capability realization, confinement and accounting observations. A verifier that
merely trusts JSON claims violates this interface's trust contract. Passing a
synthetic verifier in tests does not qualify a real provider.

OpenWarrant checks the returned facts against the selected interface and recorded
Warrant reference, contract, dispatch, stage and attempt. It checks minimum
native fields, Katana's realized capabilities and BLUT's actual registry. It
requires completed execution and any policy-required confinement, metered cost
and spend-cap observations. No other terminal state counts as completion.

Unsupported or unavailable provider checks remain UNKNOWN. Observed mismatches
and rejected seals are refused. Unknown cost remains visible even when permitted
by policy. Matching receipts confer no authorization, verification mark, human
acceptance or Warrant resolution.

## Identity and resource bounds

The capture is nonempty and within the caller's explicit byte limit before the
provider callback runs. The callback runs once; this SDK function performs no
I/O, discovery, retries or model calls. Caller-selected adapters may perform I/O.

The existing raw-byte digest checks that the provider observation describes the
captured bytes. It is separate from the provider's opaque receipt digest/seal.
No provider seal, digest domain, canonicalizer or native status vocabulary is
invented here. Providers must publish these before real integration can pass.

## Source observations

- Katana 0b0ac9dd1cbf69a2628ea214a4e841c5bd2888e1 has deterministic
  `PromptIR::canonical_bytes`, a chained Mekugi event log, and `ExecResult`
  status/session/usage. The inspected Rust source did not emit the complete
  dispatch-bound OpenWarrant receipt. The local checkout has unrelated edits;
  none were changed for this work.
- BLUT 6eedf207d2c539f65ef5506028d2e0e25e002e50 has native lineage/job
  references. Published main was 2502a4dd6385b077f21dd500851a99a6ec1795c9
  when checked. The inspected source did not emit the complete dispatch-bound
  receipt or verifier. No native status mapping was inferred.

These observations identify missing integration work. They are not provider
acceptance, live execution evidence or a cross-project agreement.

## Bounded checks

`cargo test -p openwarrant-core --test document_runtime` exercises synthetic
positive matching and refusals for wrong binding components, raw bytes, provider
identity/version/kind, missing native fields, rejected seals, capabilities,
registry, terminal outcomes, missing execution/confinement/accounting, receipt
limits and incomplete expectations. Missing support remains UNKNOWN. Separate
raw and provider identities and unknown cost are preserved on matching results.

Toolchain: Rust 1.97.1. Raw focused test and Clippy outputs are beside this file.
The full repository gate remains a separate merge requirement. No independent
verifier disposition or human signature was written.

## Remaining work

1. Agree provider wire formats, seal algorithms and native verification entry
   points at explicit participant revisions.
2. Implement real provider adapters and receipt producers with mixed-job and
   altered-seal controls against actual provider records.
3. Persist attributable captures and connect the store to current-basis
   resolution, retaining prior records on failure.
4. Run real integrations, independent checks and required participant acceptance.

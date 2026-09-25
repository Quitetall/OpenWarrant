# Read the proposed format through examples

These files are reference artifacts for RC.2 review. They are outside the live
Warrant corpus. Their `example:` identities are fictional; they contain no real
authorization, executed acceptance test, independent verification, or human
acceptance record. Current `war` does not yet implement this document adapter.

1. Read [minimal.md](sources/minimal.md): one valid small Warrant, no fixtures or
   approval records, with unknown context stated plainly.
2. Read [signup.md](sources/signup.md): one reviewable outcome, exact constraints,
   acceptance expectations, outputs, and stop conditions. Suggested implementation
   approach is background, so an agent may adapt it within the constraints.
3. Read [architecture.md](sources/architecture.md): the activation rule explicitly
   requires its definition, which contains no SHALL. The source owns that meaning.
4. Inspect [acceptance.json](sources/acceptance.json): input/output cases prepared
   alongside the Warrant. These are fixture data, not a claim that tests ran.
5. Read [ENTRY.md](expected/signup-packet/ENTRY.md): the proposed agent view carries
   the exact activation rule, its definition, required Warrant units, and a local
   link to fixture bytes. It says readiness is blocked because no execution
   permission or accepted architecture is established in this illustrative set.
6. Inspect [cases.json](expected/cases.json): expectations for unknown applicability,
   false conditions, missing dependencies, budget refusal, and later qualification.
   These cases become production conformance tests during Phase 1.

The optional design-history reference is absent and labeled absent. The unrelated
session rule is omitted from ENTRY for this declared file scope; its source bytes
remain in the package for routing/provenance checks. Do not equate supplied source
blobs with selected initial context. If scope is missing, both conditional rules
are conservatively included; cases.json records that separate expected behavior.

## What the current audit proves

Run from any directory with Python 3.11 or newer:

```bash
python3 /mnt/4tb/OpenWarrant/docs/sas/drafts/1.0.0-rc.2/examples/check_examples.py
```

The script checks this fixed corpus's TOML metadata, required units, expected
membership, exact source spans, source/blob hashes, stored canonical preimages,
and byte accounting. It checks the example against declared expectations; it is
not a production parser, condition engine, complete package validator, independent
verifier, or all-feature test driver. It cannot prove the proposed Rust features
work before they exist.

The expected packet was authored as a reference fixture. Canonical JSON bytes
were produced with the existing repository function
`openwarrant_compiler::canonical::to_canonical_bytes`, which uses the JCS library
selected by OW-ADR-0001. No replacement canonicalizer or RC.2 compiler was used.
Stored [root preimage](expected/package-root-preimage.json) and
[basis preimage](expected/basis-preimage.json) let the audit reproduce the example's
SHA-256 bindings without adding a Python canonicalizer. A production reader must
reconstruct and canonicalize those objects itself rather than trusting these
fixture-side preimage files.

The portable package itself is only `expected/signup-packet/`; preimages, source
copies, and audit script are fixture-authoring aids outside it. Reading required
context offline needs only ENTRY.md and its linked package blobs. Machine-readable
packet and manifest are canonical JSON and are deliberately compact. Source unit
spans are also listed in [source-unit-map.json](expected/source-unit-map.json).

Changing the approved format means reviewing and regenerating expected fixtures
from that new contract. Do not change expectations merely to make a future compiler
pass. Snapshot matches require independent negative/control cases in the build scope.

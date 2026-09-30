# Document SDK fixtures

This suite exercises `parse_document`, `validate_document` and exact unit access.
It is executable input, not a filename-to-verdict lookup. Run from repository root:

```sh
cargo run -p openwarrant-core --example sdk_probe -- --scope 75 --fixtures conformance/sdk
cargo test -p openwarrant-core --test document
```

`cases.json` records expected outcomes over actual bytes. The driver exits nonzero
when the observed parser/validator outcome differs. Integration tests additionally
check literal slices, offsets, resource boundaries and content mutations.

The minimal RC.2 fixture is retained byte-for-byte from the RC.3 companion's
historical `examples/sources/minimal.md`. The RC.3 minimal/ADR fixtures are copied
from `examples-footer/`. Other files are explicit positive examples or mutations
of those inputs. They carry illustrative `example:` identities, not real approvals.

T01–T10 and FOOTER-01–07 parsing/validation cases are covered. Source digests,
external reference resolution, readiness, signing, author/edit and CLI parity are
not observations made by this suite. FOOTER-08 author/edit belongs to OW-WAR-0076;
this parser is the read-side seam for its future round trip.

Context documents require at least one nonempty **binding** unit under literal F2.
Additional background units are permitted. Unknown optional extensions are retained
with warnings; unsupported required extensions leave structural validity separate
from unsupported semantic interpretation. No source grants execution permission.

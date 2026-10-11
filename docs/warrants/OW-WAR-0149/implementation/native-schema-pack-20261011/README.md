# Experimental native schema pack

Unverified implementation observations; no independent verdict or human acceptance.

`war admin schemas` now writes a separate `schemas/native-draft/` pack. It
contains six draft JSON Schemas, structural TypeScript projections, unsigned
examples and hashed manifests. The existing v1 schemas, TypeScript and pack
identity are unchanged. Contract format-basis selection is unchanged.

The public validator checks six examples, original TOML retention and manifest
hashes, and observes 24 refusals. CLI fixture checks prove repeated generation
is deterministic, existing v1 publication bytes stay unchanged, altered schemas,
types, examples and manifests are refused, and a missing native schema is
refused. All mutations occur in a disposable fixture. Plant 176 runs these checks.

The first test correctly failed because no pack existed. A later test exposed
an absent optional date-time checker: malformed dates passed the library's
unconfigured checker. The fixture now uses an explicit calendar assertion; its
rerun passed. Both failed observations remain retained beside the passing runs.

All 458 all-feature CLI library tests passed on Rust 1.97.1 at source
`fb715d1d`. Strict all-target/all-feature Clippy then reported a collapsible
conditional. The equivalent single guard at `59ab1a5b` passes Clippy with
warnings denied; the failed run is retained. A first
TypeScript test expected an unparenthesized type; inspection showed the emitter
correctly preserves its existing parenthesized syntax. The assertion was
corrected and the failed observation retained.

These checks establish portable shape validation only. A schema validator must
enable format assertions. Signatures use the original TOML bytes, not reconstructed
JSON. Actual readers still enforce byte budgets, supported versions, exact signed
fields, protected keys, permissions, presence and fresh evidence. The examples
carry explicit unsigned placeholders and never qualify an actual Warrant.

OW149 remains in progress. Workbench identities still need mapping to actual
retained Dispatches. Production signing, real operator observations, independent
qualification and participant sessions remain separate requirements. OW95 has
no participants; none are simulated here.

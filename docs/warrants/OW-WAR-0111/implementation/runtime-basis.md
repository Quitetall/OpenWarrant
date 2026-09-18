# Archive-derived runtime query basis

`war archive runtime-basis ARCHIVE --json` validates the experimental archive,
reconstructs its source IR and returns `result` with schema
`oh.war/runtime-archive-basis/v1-draft.1`. The result names the archive digest,
Warrant identity, recomputed current contract digest and retained authorization
record revision/digest/source tuples. Contract-history coverage remains explicit.
No provider process runs, no signature is inferred and no archive coverage is
upgraded. This is a comparison input for provider readers, not trusted authority.

The source-detached integration test checks exact IR contract identity, unchanged
archive bytes, preserved unavailable-history state and rejection of a substituted
archive subject. All 16 preservation integration tests pass on Rust 1.97.1, as does
all-target CLI Clippy with warnings denied.

Applying the command to the retained source-complete fixture exposed a provider
fixture mismatch: the UUID agrees, but the provider test used
`integrity.composition_revision_digest` as `contract_digest`. OpenWarrant's actual
`WarIr::contract_digest()` is a distinct digest over its defined contract view.
Existing retained provider observations remain unchanged and prove byte
preservation only; they do not establish this cross-system contract binding.
The provider fixture must use the producer's recomputed contract digest.

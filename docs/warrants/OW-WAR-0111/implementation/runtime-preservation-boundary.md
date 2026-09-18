# Runtime preservation: remaining integration boundary

Source inspected at OpenWarrant `376624a` and Knowledge Fabric `bcb7accc`.

## Available records

`openwarrant-core/src/seam.rs::KatanaReceipt` defines the provider receipt and
checks its required fields and expected dispatch digest. Its receipt digest and
runtime event-log head are provider-owned facts, not a local gate receipt seal.
`openwarrant-cli/src/resolve.rs::runtime_receipts_match_the_basis` still documents
that there is no local receipt store. Its result must not be used to assert that
runtime records do not exist in the provider.

Knowledge Fabric maps `warrant-runtime-receipts` to
`work.warrant_runtime_receipt` in its preservation import target inventory.
The preservation test exports and restores those rows together with Warrant
contracts, dispatches and action history. Its nested receipt bodies are synthetic
fixtures. They demonstrate preservation of the populated provider category;
they do not demonstrate a real Katana execution.

## Remaining work

1. Supply a provider export/resolver boundary for local non-human stage archives.
   Preserve exact receipt bytes, provider identity, Warrant/contract/stage and
   dispatch binding, and the provider snapshot identity. Do not copy a provider
   boolean into local completeness or fabricate a replacement receipt store.
2. Check both missing and mismatched bindings offline, retain all selected
   historical attempts, and distinguish no attempt from unavailable history.
3. Exercise a real no-paid-call runtime record through the provider and archive
   path, preserving its actual provenance. Existing synthetic records remain
   labeled as fixtures.

Until that path exists, `preservation/context.rs` correctly leaves non-human
runtime coverage unavailable. Source-complete human-stage fixtures and actual
OW30 selected-local roundtrips do not settle this integration gap. No change to
legacy resolution, human authorization or assurance status is made by this note.

# Candidate compact-result ingestion

The public v3 response ingest control first observed the unsupported-protocol
UNKNOWN. The new path strictly parses/preflights the whole v3 response, retains
the v2 canonical-archive/source/performer checks and validates exact canonical
model views, sibling archive bytes, projection, budget and obligation coverage
before obligation or journal writes. The journal explicitly binds protocol and
view references. A nonserializable read adapter retains the full typed binding
envelope; writes use the actual SDK v2/v3 types, preserving v2 wire bytes.
Current-work and immutable-candidate assessment share the same predicate.

Rust 1.97.1: all 57 schema-enabled verification CLI regressions and five
response-envelope unit controls passed, and all-target Clippy passed. The new
control observes v2-to-v3 raw history retention, current preparation, exact
replay, named refusal of rehashed instruction tampering, named stale-source
refusal and loss of currency after source changes, without changing prior
records/journal on refusal. Existing old-reader, prior-record, protected-source,
fixture, duplicate and independence controls remain in the suite. Unsupported
future response tests now use v4 because v3 is the implemented family; legacy
v1/v2 parsing and frozen SDK decoding are unchanged.

No real model call, production response ingestion, Warrant resolution or human
acceptance was performed. These are synthetic public-interface controls, not
project qualification or proof of backend custody. Actual model and immutable
candidate round trips and full CI remain required before claiming completion
of t-b9610. Compact --run is still not enabled.

Raw red/green logs and tests:
`/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/compact-ingest-20261016/`.

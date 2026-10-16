# Candidate compact response codec

Child work of the compact package/storage proposal in PR292. The new SDK
response decoder selects exactly verification-response/v3, rejects unknown
fields and legacy/future protocol labels, and preserves the supplied
observations and both reference sets. Preparation preflights all rows and
bindings before returning a lazy iterator; it does not copy a large subject
map for every observation at once.

On Rust 1.97.1, eight compact/legacy record and schema controls passed, as did
all 55 schema-enabled verification CLI regressions. The response schema is
registered and emitted/drift-checked by the official exporter, outside the
unchanged active pack. The CLI control plants each candidate separately and
observes the named drift refusal. The initial missing decoder was observed
before implementation. Controls reject duplicate or empty observations,
missing views, malformed hashes, blank Warrant identity, unknown fields and
legacy/future protocol substitutions. Observation and independence data remain
byte-equivalent as typed values; no confidence or disposition is upgraded.

This is metadata preparation only. CLI compact ingestion, actual view delivery,
archive/view/current-source checking, journal bindings and qualification remain
unfinished. The active reader still refuses v3; no production observation,
Warrant resolution, assurance mark or human act was recorded.

Raw checks are retained under
`/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/compact-response-20261016/`.

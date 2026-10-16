# Compact preview binding precheck

`war verify <alias> --performer <actor> --inspect-preview <absolute-file>`
checks the exact view and sibling archive without importing either, running a
verifier, or writing a verdict. It reuses the existing packet validation through
an explicit byte reader. Existing ingestion continues to read retained packets
through its original descriptor-safe store. The v2 response and stored-record
formats are unchanged.

Rust 1.97.1: all 54 verification-subject CLI regressions passed. New controls
exercise matching standalone-Git fixtures, rehashed instruction tampering,
raw archive whitespace changes despite an unchanged canonical value, stale
source inputs, refusal of ticket identifiers, and unsupported future preview
versions. The last control first reproduced ERROR for an unimplemented future
format; it now reports UNKNOWN without writing anything.

All five freshly generated OW-WAR-0003 previews passed the same check against
their archives and then-current working candidate. This is an observation at
that source snapshot, not a claim that retained packages remain current after
later edits. It is not independent verification or evidence sufficiency.

Retained raw tests and five inspection reports:
`/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/portable-review-binding-20261015/`.

Next: define the executable compact response/storage protocol in the proposed
ADR, bind every verdict to exact view and archive references, enforce view
admission and source coverage before ingestion/qualification, preserve old
readers' refusal and history, then run synthetic and production local reviews.
No production disposition, signature, assurance mark or completion was recorded.

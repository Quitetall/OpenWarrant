# OW-WAR-0084 implementation

The pure SDK captures and imports explicit legacy/current-document inventories,
checks exact byte hashes and coverage, and exports retained bytes without rewriting
signed records. Source dialect and adapter version are explicit. Historical
identity, state fields, JSON and signature payloads remain recoverable. This is
inventory integrity, not historical closure, signature validity or qualification.

A successor mapping binds distinct entry identities and the exact old/new digests
for each declared same-path replacement. It does not inherit authority, rewrite an
old delivery or generate blanket correction work. Existing dependency readiness
blocks only explicitly relevant prerequisites. Evidence availability remains the
OW83 supplied-record helper, preserving history while reporting assurance gaps.

| Case | Observation |
| --- | --- |
| T49 | Real OW-WAR-0002 historical inventory, resolution, authorization and correction DSSE retain exact bytes; no new mark |
| T50 | Corruption, missing declared blobs, unknown adapter/dialect, traversal and malformed JSON refuse; unavailable external semantics remain explicit |
| T51 | Same pathname retains both versions and exact lineage; self-successor refuses |
| T52 | Unrelated unresolved Warrant does not block; an explicit failed prerequisite blocks the named successor |
| T53 | Current RC.3, RC.2 and legacy export/import/export retain original byte maps and deterministic exported identity |

Eight public tests and ten file-backed `sdk_probe --scope 84` cases pass on Rust
1.97.1. Clippy passes with warnings denied. Independent specification and standards
reviews pass the bounded slice. Review found and repaired historical colon-name
compatibility and preallocation limits for export, blobs and JSON metadata.
Independent allocator probes observed maximum allocations of 280 bytes for a
500 KB blob under a one-byte file limit, 26 bytes for an unsupported large capture
dialect, and 40 bytes for oversized import metadata. These are bounded test
observations, not universal memory or runtime guarantees.

No filesystem migration, live storage deletion, cryptographic verification or
new authority act occurs. Original source trees and legacy signatures remain intact.
The full repository gate and required commit review are recorded after execution.

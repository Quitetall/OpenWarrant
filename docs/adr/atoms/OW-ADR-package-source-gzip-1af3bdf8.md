---
schema: oh.war/atom/v1
adr_uuid: 01a1420e-8226-7349-8277-9b5e1af3bdf8
local_alias: OW-ADR-package-source-gzip-1af3bdf8
role: adr
jurisdiction: bound
order: 100
classification: internal
status: proposed
---

# Bound a compressed full-source release capsule

## Status

Proposed packaging decision for the owner-requested unverified alpha preview.
This is not Stable qualification, SAS acceptance or authority to publish.

## Observed failure

Release run 38819931497 passed the full gate and both native builds at
7ab1e6a70fd2d1673cb4c10d2208fc34d52f8557. Both package steps refused
`Bundle payload limit exceeded`. The complete committed source tar alone is
264,755,200 bytes against a 268,435,456-byte outer payload limit. It leaves
less than 4 MB for the executable and notices. No source is dispensable merely
to make this check pass.

## Proposed representation

A new `oh.war/candidate-bundle/v2` capsule carries the complete committed Git
source as `sdk-source.tar.gz`, rather than uncompressed `sdk-source.tar`.
The inner tar preserves every original path, byte, mode, symlink and notice.
No source selection or history deletion is introduced. The outer manifest
continues to bind each payload's actual bytes and SHA-256.

The existing 256 MiB limits stay unchanged: committed source input, expanded
source tar, outer payload, compressed distribution and expanded outer tar
remain bounded at their respective layers. Source decompression must stop at
limit plus one and refuse overflow before source extraction or executable
execution. Existing tar member, sparse file, path, duplicate, link and manifest
checks remain binding. Compression does not grant trust or qualification.

Readers continue to support v1 without reinterpreting its uncompressed source
entry. V2 requires its explicitly named gzip source; malformed or unsupported
versions must not fall back to v1. The candidate remains `qualified = false`
and `promotable = false`. Native build, install, provenance, parity and owner
release gates remain required. An old published release or spent tag is never
rewritten to activate this proposal.

## Alternatives

Raising the limits changes the resource policy. Omitting history or fixtures
breaks the complete-source promise. Splitting binary and SDK distributions is
a larger delivery interface change. A bounded compressed source member keeps
the existing single downloadable artifact and full source context.

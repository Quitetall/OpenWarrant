# Current W81 package observations — 2026-10-23

Performer observations, not independent findings or secure human acceptance.
Original implementation and review history remains unchanged.

Seven Core packet tests, seven native literal cases and all three original
provider package tests passed. The external caller at provider2572894 added twelve
fresh package/filesystem cases using CRLF and Unicode input. Its source and exact
execution identity are retained; no tracked provider logic, manifest, lock or
OpenWarrant dependency was changed.

Valid packages repeat exactly and verify without their original source directory.
An incomplete package has one required context reference removed and its manifest
recomputed with the existing canonical/digest functions. It passes SDK integrity
with semantic coverage unestablished, while provider reconstruction returns
semantic-mismatch. The separate current SDK consumer independently observes that
integrity boundary through twelve calls, including the seven literal codec cases.

Entry/packet tampering refuses by digest. Publication of a different valid package
to an existing name refuses and preserves the original bytes without extra staging
paths. Traversal and symlink publication refuse. Reads refuse a symlinked entry,
an extra path and a Unix-socket entry; the latter returns source-unavailable, not
an invented proof of a regular file. An entry-size cap refuses before publication.
The original published package remains intact after the controls.

Provider selection reconstruction means coverage under the declared basis only.
Readiness stays blocked. Linux filesystem observation only; the parent capability
is caller-controlled and no sandbox against equal-privilege writers is claimed.
No power-loss guarantee beyond the shared profile, deployed trusted service,
model quality, Phase 2 exit, code qualification or human acceptance is established.

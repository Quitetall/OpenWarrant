# Full-source capsule repair candidate

Release run 38819931497 passed its full gate and both native builds at source
7ab1e6a70fd2d1673cb4c10d2208fc34d52f8557, then both package steps refused
`Bundle payload limit exceeded`. Measured committed blob bytes: 253,400,983;
complete Git tar: 264,755,200; unchanged payload limit: 268,435,456.

A real small Git fixture reproduced that exact constructor refusal in 0.661 s.
The candidate v2 representation compresses the whole source tar as one member.
All 26 release tests pass, including exact equality with Git's full tar,
source file bytes, executable flags and an internal symlink. Existing v1
capsules remain readable. Refusal controls cover nested expansion, malformed
and truncated gzip, ambiguous source members, unknown versions, unsafe tar
entries, sparse members, hidden expanded metadata, false marks and bad hashes.
The actual 256 MiB constants are asserted unchanged. This is not an alternate
source selection: no tracked source or history is omitted.

The exact guard and measured overflowing input establish the cause; speculative
compiler bisection is not needed. Full-corpus local rehearsal and fresh native
packaging/install results remain required. The proposed ADR records the new
representation without accepting a governing architecture or authorizing a
release. No Stable or human assurance act has occurred.

Raw red/green logs and all release controls:
`/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/release-source-compression-20261015/`.
Original native failure log:
`/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/alpha4-preparation-20261015/version-fix-continuation/release-failed.log`.

# W144 preview checks retained-file collisions

**Correction implemented, unverified.** A real public CLI control reproduced a
misleading preview: `war sign --dry-run` reported `authorize.would-record` for
an amendment whose actual signature path refused `authorize.retire-collision`.
The archive check was below the dry-run return.

Both modes now inspect the retained path through the same helper. Recording
still copies a missing archive before replacing authorization; preview never
creates it. Identical regular retained files stay intact. A different or
non-regular retained path refuses before authorization writes; unreadable
retention state is UNKNOWN and blocks instead of claiming it can be recorded.
Neither the record schema, archive name/hash, signed bytes nor authorization
meaning changed.

The [first red run](red.log) has 8 passes and the intended preview mismatch.
The first correction passes 9/0. Testing its new read path then reproduced a
FIFO hang, stopped by a five-second observation guard; a live child was seen
waiting for a pipe partner. The helper now inspects file type before opening
the retained path. The [final run](documented-green.log) passes 10/0, including
non-mutating preview refusal, FIFO preservation, real recording refusal,
byte-exact old authorization retention, attestation verification and its
archive-removal control, active-record separation and identical-copy reuse.

The [batch regression](batch-controls.log) passes 12/0, including no-write
batch preview, atomic failure, actual fixture signatures and exact-byte
attestation checks. These two runs each include their own positive corpus
check; they are not 22 distinct feature controls. Eleven authorization units,
all-target Clippy with warnings denied, formatting and shell syntax pass.
[Exact source and binary witnesses](observation.json) distinguish the tested
intermediate and final builds. The documentation-only clarification was rebuilt
and the complete retention controls rerun on that executable.

Fixture actors and disposable keys do not establish human approval or independent
verification of W144. The no-write observation covers the synthetic Warrant's
regular files and the retained pipe; it does not claim every file in a repository
is immutable during request preparation. Later storage errors and concurrent
changes can still prevent recording. Formal delivery reconciliation, admissible
gate evidence, independent dispositions and human resolution remain open.
No old signature, disposition, authority record or delivery pin was changed.

The owned private repository, fixture state and runtime scratch were removed.
Durable raw observations and executable snapshots remain under
`/mnt/4tb/artifacts/OpenWarrant/keep/ow144-dry-run-20261024/`.

# Native archive round trip through Knowledge Fabric — 2026-10-10

A real CPU BLUT fixture archive now passes the existing KF preservation drill.
All required source categories are retained or explicitly absent. Two independent
PostgreSQL instances and separately restored SeaweedFS storage retain populated
Warrant families, historical states, exact contract revisions and the original
version's bytes. The source services stop before target recovery. The original
two KF scenarios also passed unchanged.

The recovered OpenWarrant archive is 88,687,665 bytes, SHA-256
`cf44c3f23a7a63ed52758d867b7aac009aa19fa1ee3242c455b02405e7b0e7e4`.
From an empty directory, the candidate OpenWarrant CLI reconstructed source IR,
imported inert records, re-exported identical bytes and reconnected native inputs.
The configured external native verifier validated the recovered fixture and
rejected a changed signature and job file. A separately pinned public fixture
key digest is embedded in `reconnect-kf.py`; no source repository or input path
is read by that replay. Neither archived code nor archived authority is activated.

KF base: `3194fc896d88699b3b082afe85cf4fd922cc4eae`; candidate: `b95d5702de96ea1de2f62687f528c7b64bbceb24`.
Integration change: [Knowledge Fabric PR #29](https://github.com/Quitetall/openhuman-knowledge-fabric/pull/29).
Node 24.18.1 (CI pin), pnpm 11.21.0, Vitest 4.1.11. Test types, ESLint and
Prettier passed. The Node binary came from the official version archive, checked
against SHA-256 `d6c664df3f3f61458e8c277585571328522d705166723a7c7823a9253a4d15a0`.
Exact raw inputs, source-derived sidecars, signed fixture exports, public fixture
trust metadata and outputs live under
`/home/brianklam/Projects/OpenWarrant/docs/runtime-evidence/native-kf-recovery-20261010/`.

Failures are retained: the initial external-input route was absent; a failed
patch attempt left the same route absent; Vitest's general Buffer comparator
then exhausted a 4 GB heap on the full native archive. Its own equality helper
also reproduced exhaustion with 8 MB buffers under a 128 MB cap. The canonical
archive digest matched in a separate 1 GB process. `Buffer.equals()` preserves
exact byte comparison and refused a one-byte change under the 128 MB cap; no
input was removed or acceptance condition weakened. Only verified owned crash
and fixture resources were removed, with ownership and cleanup receipts retained.

These are fixture observations with synthetic KF actors and public test keys.
OW111 remains in progress: a real corpus sample, format adoption, independent
qualification and hosted integration gates remain. This does not resolve OW111
or accept any actual project result.

---
schema: oh.war/atom/v1
adr_uuid: 01a12668-259a-7c5c-bedd-290298b57f41
local_alias: OW-ADR-native-inputs-98b57f41
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0b4be-13af-7540-90a8-28d5c6d7ac59"
  - "war://01a0f502-4941-70a1-a446-e1eb77dff191"
---

# Retain native verifier inputs as inert archive data

Proposed experimental implementation under OW111 and OW149. This does not
adopt a stable format, authenticate a provider or change an existing digest.

A captured receipt can refer to native job files and a producer executable that
are outside the Warrant directory. Preserve their exact bytes in the existing
create-without-replacement store. A separate canonical JSON manifest uses
`oh.war/preservation-runtime-inputs/v1-draft.1`, names the exact capture by its
existing SHA-256 identity, and records each required input's role, original
repository-relative source, retained blob name, exact SHA-256, byte count and
original Unix mode. Blob names are `sha256-<hex>.bin`; manifest names are
`inputs-<canonical-manifest-sha256>.json`. Both live in the Warrant's
`native-inputs/` directory. The SHA-256 preimage is the complete canonical
manifest bytes, with no self-referential digest. Existing captures and native
receipt bytes remain unchanged.

The CLI reads an explicit versioned request containing repository-relative
public-key, expected-plan, expected-binding, producer and job paths. No native
verifier or executable path is selected from imported data. All reads use safe,
bounded descriptors, reject links and special files, and observe mode and bytes
from the same opened inode. Validate all inputs before publishing the manifest;
failed publication can leave immutable orphan blobs but no successful manifest.
Limits cover aggregate retained bytes and input count. Exact replay is harmless;
conflicting retained bytes cannot be replaced.

The initial interpreter supports the draft BLUT payload version 1 described by
Quitetall/blut PR103 at eca4ec7db2adea3a12f76243311c643e84cbb472. Retain the
32-byte public key, supplied plan/binding, exact producer binary and every native
job file declared by the receipt, including the registry and lifecycle data.
Check provider-defined BLAKE3 byte identities against actual bytes, complete
path/size/mode tables and exact captured Dispatch binding. Preserve the native
canonical input bytes; do not recreate BLUT plan canonicalization. Supplied
plan JSON must match retained native plan JSON, and native binding data must
match the captured Dispatch and run identifier. Unsupported shapes stay
unavailable. OpenWarrant interprets protocol facts independently; it does not
vendor the provider's AGPL implementation. The BLAKE3 implementation dependency
offers Apache-2.0 licensing independently of the provider.

Archive runtime-reference coverage may become retained only when the exact
capture, Dispatch, journal sources, supported input manifest and every required
blob reconnect. A later-retained manifest can describe an earlier identical
Dispatch; report its actual path and never invent earlier retention or execution.
Missing inputs, conflicting manifests, changed bytes and unknown versions remain
visible. Retained input coverage does not establish native signature validity,
execution eligibility, trusted key custody, confinement, spend or assurance.

Whole archive import remains private inert data with its existing permissions.
Original modes are metadata, not an instruction to chmod or execute imports.
A caller may reconstruct a native data workspace under its own policy and choose
an independently trusted verifier/key; imports never activate archived authority.
Real source-detached native verification and KF round trips remain required
observations, separate from format adoption and independent qualification.

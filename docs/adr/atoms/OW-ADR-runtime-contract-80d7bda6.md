---
schema: oh.war/atom/v1
adr_uuid: 01a1260c-6f6a-729f-b0a2-0faa80d7bda6
local_alias: OW-ADR-runtime-contract-80d7bda6
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0b4be-13af-7540-90a8-28d5c6d7ac59"
  - "war://01a0f502-4941-70a1-a446-e1eb77dff191"
---

# Retain exact runtime contract source snapshots

Proposed experimental implementation decision under OW-WAR-0111 and OW-WAR-0149.
It is not accepted architecture or a new stable format. Existing capture, receipt,
Dispatch, contract and archive wire bytes and digest domains remain unchanged.

A runtime capture needs a reproducible contract even when a Git commit did not
retain generated IR. Retain a separate canonical JSON record with explicit schema
`oh.war/runtime-contract-snapshot/v1-draft.1`. It contains the lowered IR and exact
manifest, atom and optional scope bytes, each with ordinary SHA-256 and canonical
base64. The IR carries source identities, composition metadata and optional SAS
pin. Reconstruct from those exact inputs with the existing compiler and compare
the complete contract digest with the recorded Dispatch. Unsupported versions
remain unavailable; malformed supported records refuse.

Store snapshots under the Warrant's `runtime-contracts/contract-<digest>.json`,
using the existing contract digest's lowercase SHA-256 hex. Retain through the
existing create-without-replacement store. A failed later capture publication may
leave a valid standalone snapshot; it is not evidence that execution completed.
No timestamp or invented authority is added, and no old capture is rewritten.

Archive queries may use a retained snapshot when the selected historical sources
have no generated IR, only after every manifest, atom and scope byte matches that
historical source selection. Record the snapshot's actual source path. Never claim
it existed at the historical commit merely because it reconstructs that contract.
Refuse ambiguous matching metadata instead of choosing the newest record. Missing
or mismatched historical sources remain visible as unavailable reconstruction.

A checksum and reconstruction establish byte identity and contract interpretation,
not provider authenticity, original historical compiler custody, active authority,
execution eligibility or assurance. Required native records, independent evidence
and secure human acceptance remain separate. Stable promotion needs the required
human format decision. Proposed ADRs do not become binding merely by compilation.

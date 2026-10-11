---
schema: oh.war/atom/v1
adr_uuid: 01a12868-f073-7667-8a9e-17a4e8b574f1
local_alias: OW-ADR-native-policy-e8b574f1
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0f502-4941-70a1-a446-e1eb77dff191"
---

# Bind native acceptance to protected policy observations

Proposed implementation under OW149; not stable format adoption or human acceptance.

A native acceptance response must bind the policy under which its evidence was
assessed. The draft `oh.war/runtime-native-policy-binding/v1-draft.1` object
names the authenticated repository and collector, the existing authority revision
digest, the existing active signed-enrollment digest, and exact protected
`runtime-host.json` and `runtime-native.json` byte identities. Configuration
identities are `sha256:<lowercase hex>` over complete raw file bytes. Authority
keeps its existing revision digest domain. The
`active_enrollment_digest` keeps the authority store's existing SHA-256 identity
over `runtime_collector::Signed::encode()` bytes, including signatures; it is
not `Enrollment::digest()`, which uses a different domain. There is no new combined
digest, canonicalization, or alteration to existing signature preimages.

The host derives this object from protected files and an authenticated active
enrollment. It never trusts a receipt's principal labels. All reads retain the
existing safe descriptor, bounded input and execution-account protection checks.
Before publishing or reusing a binding, recheck the original host and native
configuration bytes, current authority revision and active enrollment. Changed
observations refuse reuse; unavailable observations remain unknown. An empty
selection still requires these checks and grants no execution or assurance.

A held assessment must recheck policy before exposing a verifier. The verifier
also checks around actual execution. These checks are not atomic writer or launch
fencing and must not be described as such. A public serialized binding alone is
not authorization; consumers must obtain and compare a freshly authenticated
host observation. Binding fields cannot substitute for signature verification.

Versioned signed response, ingestion, persisted-record readers and human signing
must subsequently bind this policy object together with exact task, selected
captures, assurance case and artifact snapshots. The legacy v1 response must
remain compatible and must not be presented as native evidence acceptance.
Until that path exists, native response ingestion remains explicitly unavailable.

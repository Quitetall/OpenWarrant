# Shared portable package contract

Revision 1 implementation draft, shared OW-WAR-0081. OpenWarrant owns this
contract and SDK wire/integrity checks. LAMU owns deterministic package construction,
offline semantic verification, and the explicit filesystem shell.

`build_package` consumes a captured snapshot and explicit task compilation basis.
It emits ENTRY.md, canonical packet.json, canonical manifest.json, and full original
blobs named by raw digest. All parsed routing documents remain available for offline
selection; unselected opaque candidates may remain descriptors without blobs.
Every emitted source must pass supplied disclosure facts. No network or model call
occurs. Master views are not task packages.

`verify_package` first uses SDK integrity checking, then reconstructs source indexes
from original blobs, reevaluates selection and projection, and compares exact packet,
ENTRY and blob membership. Rehashing an incomplete package cannot establish coverage.
Coverage means context selection under the declared basis, not execution permission,
independent code verification or human acceptance. Task readiness remains blocked
until record evaluation is integrated.

`read_package` takes an open directory capability, reads only defined root files and
one blobs directory, and rejects symlinks, nonregular files, extra paths and excessive
resources. `publish_package` verifies before writing, stages under a private fresh
child directory, and atomically publishes to a fresh name with no replacement.
Existing outputs survive failures. Parent directory capability must be controlled by
the caller; this is not a sandbox against other writers with the same OS authority.
Staging paths and process identifiers never enter package bytes. Linux execution is
proven locally; macOS remains a CI/runtime obligation. Power-loss durability beyond
synced payloads and staging directories is not claimed.

Package JSON, entry, source, file-count and total limits apply before relevant
copies or writes. A failed operation returns no successful partial package. Earlier
caller-held packages and historical evidence remain unchanged.

Provider profile: `lamu.openwarrant.package/1`, exact SDK pin inherited from source
profile. Participant receipts bind provider revision, contract digest and fixtures.
No duplicate Warrant, signature, policy or assurance is created by this integration.

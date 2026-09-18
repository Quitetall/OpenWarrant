# Current source-complete archive through Knowledge Fabric

The fixture was generated with OpenWarrant producer commit
`3441573fea9dc82fbd34a36fa71e88f689518db5`. Exact producer and archive digests
are recorded in `conformance/fixtures/preservation/kf-source-complete-identity.json`.
The original disposable source repository was removed. Older fixtures remain intact.

Knowledge Fabric's preservation test passed on Node 24.21.0 and pnpm 11.21.0,
using real PostgreSQL and MinIO containers, source service shutdown, database
export/restore and copied storage volumes. The test uses the current archive and
checks all fourteen coverage categories are retained or explicitly absent.
It also retains populated provider-owned Warrant tables and nested runtime receipt
fixtures. These synthetic receipts prove record preservation, not agent execution.
No model was called. Provider base: `876efc7fc1248bc1dab666375e18b634ebbd834e`;
changes are on `codex/ow111-source-complete-roundtrip`.

The bytes recovered through the provider storage SDK match the source fixture.
The current OpenWarrant producer inspected, imported and re-exported those bytes
from a new empty working directory. All three commands returned zero. Re-export
SHA-256: `b6318636c97fa0fbae729add05b13a5539e7c7388d5f37f2152a2fc3765e2fbb`.
See `kf-source-complete-import-proof.json` and `kf-source-complete-roundtrip.log`.

Provider test formatting, ESLint and test TypeScript compilation passed.
Completeness remains selected local current/history scope. This does not establish
all external runtime coverage, human acceptance, independent qualification or
Stable format approval. OW-WAR-0111 remains unfinished.

Provider extension committed as `bcb7accc`; draft PR: https://github.com/Quitetall/openhuman-knowledge-fabric/pull/4. Hosted checks are pending.

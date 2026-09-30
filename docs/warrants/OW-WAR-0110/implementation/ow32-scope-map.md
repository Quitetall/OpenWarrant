# OW32 scope preserved by OW110

Source inspection: OpenWarrant ce6ef1fb6a5613ed57cc4d6637b81dc42d24f20f.
This maps historical obligations without changing their dispositions or signatures.

| OW32 obligation | Current evidence | Remaining gap |
| --- | --- | --- |
| OBL-001: IR, manifest, Dispatch, Submission and gate schemas generated; existing corpus validates | `schemas.rs::entries` generates 15 families including war, manifest, stage-dispatch and stage-submission | No gate family in that list. Rendering and compilation are not corpus-wide JSON Schema validation. Full stated observation remains open. |
| OBL-002: FormatBasis.digest matches assembled pack | `render_all` assembles sorted member hashes and transitive digest. `schema_pack_version` checks embedded pack version. | `FormatBasis` in compiler/ir.rs has package/version/root/profile/SAS pins but no pack digest field. OW110's separate projection manifest does not satisfy this historical field requirement. Resolve through explicit governed format amendment or accepted reconciliation; do not silently change contract digest semantics. |
| OBL-003: generated schema drift refuses | Existing `89-schemas.sh` checks edited/missing JSON schemas and changed pack; OW110 adds retained TypeScript drift refusal observation | Preserve existing JSON behavior and exact-source final gate. TypeScript drift does not substitute for missing corpus semantics. |
| OBL-004: generated TypeScript exists and KF imports it | Fifteen structural declarations; KF package build and actual submission validator import; public positive/negative compile fixtures; 24 runtime tests | Hosted integration checks and merge still pending when this map was written. Human assurance remains separate. |

OW32's work order freezes schema pack identity/version and requests version
checking, pack assembly, generation and KF consumption. OW110 preserves current
JSON bytes, pack identity and algorithm. It does not close all OW32 scope, add
missing schema families, or equate structural types with SAS conformance.

Additional existing omissions visible in `schemas.rs`: amendment and SAS-revision
families carry source comments about resolved-file corrections. Those comments
are historical leads, not proof of current authorization; inspect current pins
and correction records before any future source change.

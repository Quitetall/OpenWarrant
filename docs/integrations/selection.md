# Shared selection contract

Revision 1 implementation draft. Canonical owner: OpenWarrant; shared work
OW-WAR-0079. LAMU retains this document's digest and implements selection in its
optional `lamu-openwarrant` crate. No signature or assurance is granted.

The provider consumes a captured Snapshot, explicit task path or master mode,
validated effective condition inputs, an explicit digest allowlist and access
basis references, include_optional, and positive selection limits. No ambient
files, models, discovery or policy inference participate. Empty allowlists deny
all disclosure; empty access basis is permitted only with a reported limitation.

Task selection starts with every binding task unit and every applicable required
pointer across all captured sources. UNKNOWN conservatively includes the target
and its dependency closure, with a warning. Optional pointers stay in a catalog;
include_optional adds their exact content and mandatory dependencies. Background
dependencies are required verbatim; raw references retain whole bytes. Raw path
references do not activate parsed unit dependencies; use `#*` to select all parsed
units and their metadata semantics.

Closure uses stable captured digest/unit identity, preserves all distinct direct
inclusion reasons, and terminates cycles with a member diagnostic. Required roots
and dependencies win over optional reasons. Missing required targets, selected
conflicts, competing required revisions and access denial prevent complete output.
Results preserve exact bound references and authored kind. They never grant
execution readiness or human assurance. Failed calls do not mutate a prior result.
Master mode selects all units and opaque sources; it grants no task readiness.

The public result includes deterministic selected references, pointer catalog,
diagnostics and context_complete. Diagnostics sort by code/path/unit/pointer;
selected references by digest/unit; reasons by kind/path/id. Errors prevent
context_complete. Unsupported required extensions fail during source validation.

Default selection limits: 65536 selected units, 262144 processed dependency edges,
262144 inclusion reasons, 1048576 traversal visits, 64 MiB charged working
allocation and 64 MiB serialized result. Working allocations are conservatively
charged before copying; their effective cap is also at most eight times the
output limit. Traversal visits bound repeated edge scans and cycle walking. Custom positive limits are
explicit resource controls. No partial result is returned on quota exhaustion.
T21–T26 exercise exact closure, duplicate reasons, cycles, missing/conflicting
members, externally routed rules, and optional-to-required refusal. Access, limits,
UNKNOWN inclusion and unchanged prior results receive additional public tests.

Participant writes: OpenWarrant shared contract, integration driver/fixtures and
0079 implementation evidence; LAMU selection module/tests and participant pointer.
This profile is an in-process compiler primitive. Full F5 compilation, task entry,
portable packages and readiness integration belong to later staged work.

# Explicit SDK compiler adapter

Implementation profile `oh.war/compiler-adapter/1`, OW-WAR-0086. This is a typed
in-process Rust interface; no installed network endpoint or stable ABI is claimed.
OpenWarrant owns this shared contract. LAMU supplies the semantic compiler.

The caller chooses exactly one `CompilerProvider`. `info` supplies explicit profile,
compiler/build identity and supported capabilities. `prepare` borrows the typed F5
basis, takes captured original blobs, required capabilities and positive bounds,
checks source identity, task Warrant/role and bounded structure, then freezes the
request. Borrowing prevents rejection of hostile nested input from recursively
dropping data owned by the caller. No unsigned claim grants execution permission.

Capabilities are task-package and offline-semantic-check. The current profile
requires task mode; master projections remain the provider's separate public API.
Unknown profiles, unsupported capabilities, unavailable provider, invalid request,
malformed response, basis mismatch and explicit provider refusal are distinct.
No failure causes automatic fallback to another compiler, model or policy.

The response carries unchanged provider identity/capabilities, exact basis digest,
portable package files and optional provider semantic evidence. The SDK checks
package integrity and equality to the requested basis. Semantic evidence names its
compiler, basis, package root, method and coverage result. A requested semantic
check must return that evidence. It remains a provider statement, distinct from
local SDK integrity and from human assurance; SDK validation never promotes it.

`audit_context_package` is a second minimal SDK consumer. It reads a bounded JSON
map of package files from stdin, checks integrity and reports exact identities.
It does not link LAMU, use a database, contact a service or claim semantic coverage.
Actual provider integration must exercise this consumer with generated package
bytes plus changed-byte refusal, alongside all prior provider cases.

This profile is an implementation candidate. Full Phase 2 exit also requires the
Phase 1 exit, record/readiness exchange, legacy preservation and complete declared
integration cases. A passing adapter mock or package audit alone cannot satisfy it.

## Current SDK record and preservation exchange

The integration driver also passes the actual provider package through the current
SDK CLI. A local observation names its exact basis and package root. Without
caller-established trust, this record cannot clear an explicit passing-check gate.
A record for another package root refuses. No compiler statement supplies a human
signature or changes the meaning of the provider packet's readiness field.

The driver imports the legacy preservation fixture, retains its complete original
path structure under `archive/`, and captures it with the current task document.
Export/import must preserve every byte. Recompiling the restored task through the
real provider must produce identical package bytes; corrupt preservation refuses.
This is an SDK consumer exchange around the existing provider profile, not a new
LAMU wire contract or an assertion of historical signature validity.

The record subject uses a Contract-domain digest of the retained normalized
candidate contract. The compile-basis digest and package root remain separately
named identities. The candidate adds a bounded package-integrity expectation for
this test and does not claim to be an approved implementation contract.

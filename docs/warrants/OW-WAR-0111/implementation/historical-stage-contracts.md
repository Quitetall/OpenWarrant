# Historical stage contract reconstruction

The producer runtime query now adds an optional `contract_binding` to each stage
declaration. Current declarations use the verified current IR. Historical
declarations use the IR retained at that same snapshot's `generated/WAR.json`.

The producer loads the exact snapshot manifest and atoms, retained scope where
available, and recorded SAS pin. It lowers these sources again and compares the
contract digest, revision and IR API version with the retained IR. A successful
binding names its revision, digest and exact retained IR source/digest. Missing
sources, missing historical IR, stale IR or unsupported historical semantics leave
the binding null and add an explicit unresolved reason. No newer snapshot supplies
missing historical bytes.

This is content reconstruction, not signature authentication, acceptance or
execution qualification. The provider still needs to consume these exact
revision/digest bindings before historical stage matching is complete.

Validation: Rust 1.97.1 preservation suite passed all 18 tests; all-target CLI
Clippy passed with warnings denied. Historical fixtures regenerate IR for two
commits, then change a shared source without regeneration for two more. Only the
consistent snapshots receive reconstructed bindings. Current binding also matches
the producer's reconstructed current contract digest.

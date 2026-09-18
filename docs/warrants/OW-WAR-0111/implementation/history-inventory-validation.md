# Offline history inventory validation

Inspection now checks the history descriptor's schema, source boundary, supported
selection, commit identities, commit count, modes, and unique commit/file entries.
Every declared record must exist at its exact source-derived path. Every retained
history record must appear in the descriptor. Every historical manifest's explicit
atom reference must appear in that commit's file inventory. A missing descriptor
cannot accompany retained history records. Legacy descriptors without the later
shared_atom_paths field remain supported.

Validation: twelve integration tests and all-target CLI Clippy passed under Rust
1.97.1. Tests recompute outer digests after duplicating commits, removing inventory
entries or redirecting source paths; inspection refuses each. Source-detached
positive cases still pass. Inspection of the retained real OW-WAR-0003 shared-history
archive passed; its report is retained alongside this note.

Limits: this validates consistency of retained data, not Git object hashes,
authenticated ancestry, complete upstream history, or whether an attacker removed
both an unreferenced record and its inventory entry. No human assurance or complete
preservation claim follows. Full category reconciliation remains open.

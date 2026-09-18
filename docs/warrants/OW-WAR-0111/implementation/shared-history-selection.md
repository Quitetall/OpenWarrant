# Include shared-atom-only changes

History selection now expands from Warrant-path commits to commits changing each
explicitly referenced shared atom. Newly discovered historical references expand
selection until no unseen commits remain. The union is bounded at 256 commits;
path count, record count, byte size and subprocess limits still apply. Selection
is deterministic and the history descriptor records shared atom paths. Source
bodies are read from each selected commit where its Warrant manifest exists.

This closes the selection gap described in historical-shared-atoms.md. It does
not include other Git refs, unreachable objects, external providers or unresolved
bound URIs. Commits before the Warrant existed may contribute commit metadata;
no Warrant body is invented there. Historical completeness and authenticity remain
separate from retained bytes.

Validation: twelve integration tests and all-target CLI Clippy passed under Rust
1.97.1. The historical shared-atom fixture now includes two commits changing only
the shared body; both exact versions survive source removal. Its missing-source
refusal still passes. Real OW-WAR-0003 captures 21 commits and 448 records, with
its shared ADR path recorded; that repository sample has no additional shared-only
commits beyond its Warrant selection. No independent verdict is claimed.

# Execution worktree context checkpoint

Unverified performer observations for source commit
`f0e7cd8d0affbf283d6b32dc67ed5fa3812925e3`. No human acceptance,
independent verdict, native provider qualification or authority activation.

The original project owns the contract, authority and retained Dispatch journal.
The SDK optionally selects a clean committed worktree of that same Git repository
as its context source. Contract basis must match. Source bytes and holder revisions
bind the selected checkpoint. Source changes during compilation, changed contracts,
dirty workspaces, unrelated repositories and stale bundle checkpoints are refused.
This does not provide isolation against an operator able to race filesystem writes;
the configured harness remains responsible for execution confinement.

The web executor passes its own Warrant worktree through both public SDK commands.
A real configured fixture worker completes two stages: the second receives updated
committed context while the original project HEAD remains unchanged. Context delivery
alone never authorizes execution or establishes native assurance.

Observed checks:
- 14 Dispatch CLI integration tests passed.
- Strict CLI Clippy passed with warnings denied.
- The staged web checkpoint test passed (1 test, 1.231 seconds).
- Shipped conformance uses actual CLI/Git: checkpoint capture and original-journal
  retention passed; stale checkpoint, changed contract, dirty source and unrelated
  repository each produced the expected refusal without an output bundle.
- The focused governing-question test passed (1 test, 56.315 seconds).
- The complete local web suite did NOT finish: timeout status 124 after 600 seconds.
  Its partial log contains a governing-question error; the focused rerun passed.
  No complete local web-suite pass is claimed. Hosted regression gates remain required.

The initial public CLI and staged web tests observed the missing source option and
old HEAD-equality restriction. Conformance also exposed a real process/JSON status
mismatch: invocation failures exited 1 while reporting 2. The formatter now reports
the actual selected process status; process exit semantics are unchanged.
Test-construction and fixture failures are not counted as observed refusal controls.
Compressed raw logs and their uncompressed hashes are retained beside this note.

Fresh runtime-linked resume/repair scenario coverage, actual provider seals,
production custody/spend/confinement, independent qualification and human observations
remain separate work. OW149 remains in progress. OW95 has no real participants.

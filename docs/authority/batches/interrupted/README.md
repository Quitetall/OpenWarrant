# Interrupted batches

A batch here was signed by its signer and then **not recorded**: the acts it
lists are not in effect, and nothing in the corpus reads these files.

## B-20260924T080457Z-3d23881b

27 re-authorizations (revision 2 of OW-WAR-0112..0140, excluding 0126 and
0128), signed by Brian Lam on 2026-09-24 at 08:04:57Z. The owner signed while
the performer (`claude`) was running the conformance battery in the same
worktree. The battery restores `docs/warrants/` and `docs/authority/` with
`git checkout` between plants, which reverted every record the batch wrote to
a tracked path — the new responses, the authorizations, the journals —
leaving only the untracked batch document, its signature and its envelope.
The batch lists each response by sha256; those bytes no longer exist, so this
signature covers nothing in the tree.

Kept, not deleted: a signature a human gave is evidence of what was attempted.
The acts were signed again from a clean tree. The performer's fault: the
battery must never run where the owner signs.

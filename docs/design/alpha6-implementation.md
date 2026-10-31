# Alpha.6 implementation

Owner-directed implementation plan, 2026-10-31. Tracked as `t-8956e`.
This is implementation scope, not formal acceptance of a format edition,
independent verification, or a release receipt.

## Work model

New work uses one expandable Markdown Warrant with a stable ID. Items and
stages track progress. Events trigger checks; every required current result
must pass before its target advances. Optional results are reported. Ordinary
work can finish unverified. Configured approval requirements remain binding.
Watched regressions reopen unaccepted work; accepted versions keep their
history and receive linked repair proposals.

The daily interface is `init`, `create`, `start`, `done`, `status`, `check`.
Bare `war` is the human TUI. Earlier spellings remain compatible. Advanced
operations stay in `plan`, `sign`, `evidence`, `view`, `admin`. `start` gives
humans a brief and agents a complete required context packet. The reference
compiler is deterministic; SDK providers may enrich the context. Custom
checks use the existing command runner; embedded Lua is deferred.

## Delivery and evidence

1. Preserve all branch refs and authored dirty files before reconciliation.
   Merge useful completed work by PR. Archive superseded histories. Retain
   active, dirty, foreign or uncertain workspaces.
2. Add a fast PR tier beside the full checkpoint gate. Require the new green
   status before retiring the previous PR requirement. Full gates still run
   before tags and downstream pins, nightly and on demand.
3. Update the candidate standard, schemas, examples and conformance cases
   before implementing new normative semantics. Keep edition status truthful.
4. Implement one evaluator/context/admission path and finish W35's read-only,
   answer-aware interview. Question identity includes its intake or Warrant.
5. Derive and expand path scopes within existing permissions. Refuse overlaps
   in one checkout; permit isolated worktrees with integration warnings.
   Record session/workspace/base identity. Lease expiry alone proves no writer
   stopped. Inventory worktrees; cleanup is dry-run first and conservative.
6. Import milestone tables through existing import infrastructure. Preserve
   originals and provenance. Warrants own subsequent work; repeat imports
   retain IDs and propose changed rows without overwriting edits.
7. Correct onboarding and agent guidance. Verify human and JSON/MCP loops,
   Linux/macOS packaging, and installation. Publish alpha.6 only from the
   exact main revision that passed the full gate.

Tests must cover partial, failing, unknown, stale and duplicate results;
context completeness; shared versus isolated writers; cleanup preservation;
idempotent/conflicting imports; reused question IDs; and legacy compatibility.
Readable task/role names accompany stable IDs and never grant authority.

## Consumer feedback

The WMD wishlist reports overlapping writers, abandoned worktrees and work
tables invisible to the tracker. Those are alpha.6's P1 integration cases.
Current source already has a project registry, generic executable gates,
renewable leases, worktree execution and idempotent imports from three tools.
Successful `go` runs remove their own worktrees. Extend those facilities;
do not reimplement them or use top-level help absence as proof of absence.
Cross-repository queues, commit attribution and reusable golden comparators
remain later scope. Their requirements are retained, not reported delivered.

The initial preservation bundle and dirty-file manifests live under
`/mnt/4tb/artifacts/OpenWarrant/keep/alpha6-20261031/`. Git history remains
the authority for merged code; preservation copies make no assurance claim.

Alpha.6 is a prerelease. Outstanding human studies, independent qualification
and formal acceptance remain outstanding, and Stable 1.0 is not claimed.

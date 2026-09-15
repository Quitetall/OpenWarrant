# Remaining work

Status recorded 2026-09-14 in the SDK amendment checkout. The
[candidate roadmap](../sas/drafts/1.0.0-rc.3/migration-map.md) defines intended
scope. The [generated Warrant overview](../warrants/generated/WARRANT_OVERVIEW.md)
reports actual legacy records: 87 total, 29 recorded resolutions, 58 unresolved.
Unresolved records do not mean 58 unfinished implementations, and old resolutions
do not award the new assurance mark.

## Documentation and publication

- Review and publish the amendment. The new README, banner and diagrams remain
  off `main`; remote `main` was checked at `651ea2d002e4e61dbaddb9262b3e8959a110ea8d`.
- All nine legacy conflicts are reconciled. The owner confirmed the README and
  xtask corrections through the human terminal on 2026-09-14/15 UTC; old responses
  and SSH signatures are retained unchanged. Seven OW-WAR-0068 mismatches are new
  candidate deliveries with original manifest and provenance preserved. Candidate
  registration does not establish verification or resolution. Regenerated corpus:
  846 passes, 88 warnings, zero errors. See the [review packet](amendment-review/README.md).
- Complete required external commit review and the actual SAS adoption act for
  the exact candidate when seeking formal adoption. Neither is implied by draft
  checks. Routine in-scope unverified coding has no blanket signature prerequisite.
- Exercise skill invocation in fresh supported harnesses. Static checks and
  prepared prompts do not establish behavioral success.

## Build order

1. **Phase 1: standard and SDK.** The OW-WAR-0075 parser candidate is integrated
   from its isolated worktree. It passed 493 existing core tests, 13 new public API
   tests and 22 real-file cases. Independent review reproduced and then confirmed
   fixes for a depth-limit bypass and incorrect NUL diagnostic span. See
   [implementation notes](../warrants/OW-WAR-0075/implementation/sdk-parser-notes.md).
   The [integrated gate](amendment-review/integration-results.md) passes 12/14
   steps with all 740 Rust tests passing; that earlier run preceded the owner's corrections. The remaining clean-tree
   battery run and commit publication are tracked in the review packet; this is
   unverified work, not signed completion. Build OW-WAR-0076
   authoring; the SDK subsets of 0077 source types, 0078 condition syntax and 0081
   integrity; 0083 records/assurance; 0084 compatibility; 0087 CLI parity; then
   0085 Phase 1 exit. Follow actual input dependencies, not numeric order alone.
2. **Migrate the documentation.** Original sources are now retained in the
   [archive](../../archive/legacy-20260914/README.md). Create and validate new
   OpenWarrant documents derived from those sources, with section-level mappings.
   Archive copies and compatibility wrappers alone are not completed migration.
   Keep required live legacy paths until validated replacement supports relocation.
3. **Phase 2: provider integration.** Complete provider portions of 0077–0082,
   0086 integration contracts and 0088 ergonomic consumers. Prepare an exact shared
   OpenWarrant/LAMU contract with participant scope and real interoperability tests.
   Compiler internals remain provider-owned.
4. **Phase 3: workflow.** Draft concrete webapp and real-user demonstration
   Warrants against working interfaces. Exercise the complete workflow, context
   delivery, questions, stops, recovery, progress and optional human qualification.
5. **Phase 4: hardening and release.** Complete 0089 packaging, 0090 qualification
   and 0091 Stable publication, including workflow evidence and the owner's
   explicit release act. Stable 1.0 has not been published.

The main product decisions are sufficient to start Phase 1. Later integration
details still require evidence from working interfaces. Signed OW-WAR-0074 remains
on its original subject; changed adoption scope requires an explicit successor.

## Archive evidence

The preservation capture contains 1,850 baseline files with original relative
paths, 16 worktree snapshots with 19,682 entries and reachable Git history.
Byte/path checks and restoration of captured refs and baseline files into an empty
Git repository passed. Changed-original and changed-manifest probes were refused.
Signature authenticity was not assessed. The archive index states coverage limits;
preservation is complete for that captured selection, not a claim that all external
evidence or all documentation conversions are complete.

Archive checks use explicit runtime refusals, including under `python -O`. Eight
black-box cases cover valid input, changed original bytes, changed manifest bytes
and a missing snapshot member in both interpreter modes.

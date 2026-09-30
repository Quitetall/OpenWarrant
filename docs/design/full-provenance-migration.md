# Full-provenance migration

Migration target: preserve original nonconforming documentation unchanged in an
archive, then create new conforming OpenWarrant documents from its content. New
working documents retain links to exact original sources, relationships and actual
authority. The owner clarified this direction on 2026-09-14. This note prepares
the rollout; it does not report a completed migration or change signed contracts.

The user-facing entry is the [war-migrate skill](../../.claude/skills/war-migrate/SKILL.md),
invoked in chat as `war migrate`. It covers existing system documentation as well
as earlier OpenWarrant editions. This note details OpenWarrant's own rollout.

Read this when implementing OW-WAR-0084 or preparing a repository cutover. The
[RC.3 migration map](../sas/drafts/1.0.0-rc.3/migration-map.md) remains the source
for build ownership; the [SDK contract](../sas/drafts/1.0.0-rc.3/sdk-contract.md)
defines the public boundary.

## Current limits

The [legacy archive](../../archive/legacy-20260914/README.md) now preserves readable
baseline documentation, separate current-worktree variants and bundled reachable
Git history. Original relative paths remain intact. Captured bytes and paths have
been checked; signature authenticity and full SDK migration have not. The archive
index defines capture limits and remaining live compatibility paths.

The current CLI's `war migrate` imports legacy ADR files. It does not migrate the
Warrant corpus to the RC.3 format. OW-WAR-0075 supplies document parsing;
OW-WAR-0077 supplies source identities; OW-WAR-0083 supplies record semantics;
OW-WAR-0084 supplies legacy preservation and successor mapping. Implement and
test those seams before converting production records.

Records exist across branches and worktrees. The earlier OW-WAR-0074 source-set
worktree contains authorization revisions, signed responses, retained revision
files and evidence absent from the SDK amendment checkout. Selecting the newest
checkout alone would omit real history. A modification time cannot decide which
record is authoritative.

The initial local discovery lists registered worktrees, shared Git refs, dirty
paths and hashes of current document files. It excludes ignored files, historical
tree contents, reflog-only objects and external evidence. It is non-atomic and is
not a backup, signature check or proof that all records were found.

## Preservation and representation

Keep an immutable original alongside its new representation. Each mapping needs
the original repository identity, path, schema, revision/content digest and the
new record identity. For uncommitted records, retain captured bytes and mark them
uncommitted; do not assign them the checkout's commit as their content identity.
Record converter/version, conversion inputs, mapping decisions and output digest.

An adapter may expose an old record through the new SDK for compatibility; that
alone does not produce the new documents required for completed migration.
Its original signature continues to cover only its original subject
and digest domain. A new representation or successor does not inherit that
signature. Preserve original status and unsupported fields as source data; expose
unknown interpretation explicitly. Historical resolution is not automatic
qualification under the new assurance baseline.

Keep conflicting versions as separate identified inputs until their relationship
is established. Merge identical bytes by content identity while retaining every
source occurrence. Preserve deleted/superseded records that are part of the
declared historical scope. A vector index is a rebuildable retrieval aid, never
the only retained copy or the record inventory.

## Rollout and completion checks

1. **Capture sources.** Inventory branches, tags, worktrees, dirty/untracked and
   relevant ignored records, historical trees and referenced external evidence.
   Preserve Git history and non-Git bytes in a durable archive. Produce a manifest
   with explicit coverage, unavailable inputs and conflicting identities. This
   step completes when the declared source scope is captured and restore checks
   reproduce its bytes; missing sources remain visible.
2. **Build compatibility.** Implement the parser, source identity, record and
   legacy adapter seams under the existing Warrants. Fixture cases must cover
   retained signatures, amendments, corrections, evidence, resolutions, missing
   evidence, unsupported schemas and conflicting revisions. This step completes
   when both successful imports and required refusals are observed.
3. **Dry-run conversion.** Write a separate output tree and old-to-new mapping.
   Account for every inventoried record as represented, preserved opaque or
   unresolved. Compare counts and relationship closure, not only output hashes.
   Verify historical signatures against their original subjects and report
   missing keys or unavailable evidence separately from invalid signatures.
4. **Test repeat and recovery.** The same inputs and converter version must yield
   the same semantic output and mapping; attempt timestamps may differ. Repeat
   runs must not duplicate acts. Interrupted runs must resume or leave the old
   corpus intact. Reject output tampering, lost fields and dropped relationships.
5. **Review and cut over.** Independently review the preservation report and
   remaining interpretation gaps. Select one current write path, retaining old
   records for read access. Check representative old records through the new SDK
   and CLI, plus fresh unverified work. Completion requires complete accounting,
   reproducible retained bytes, validated new target documents with exact archived
   source/section mappings, and working reads for supported historical schemas.
   Unresolved conversions remain incomplete. An opaque archive or compatibility
   wrapper alone is preservation, not completed document conversion.

Repository cutover and archival tooling may need a successor or additional
unsigned scope beyond OW-WAR-0084's library seam. Migration does not authorize
rewriting signed scopes or deleting old storage. Human signatures are needed for
actual human assurance acts, not routine preparation or unverified implementation.

Independent Phase 1 development can proceed while this path is built. Full
migration cannot be a prerequisite for writing the migration implementation.

# Knowledge Fabric preservation integration boundary

Inspection date: 2026-09-18. Provider source inspected at
`798094c6ad26316d0747dbdaf4d286fbb595d373`. This is source analysis, not a
successful cross-system restore or an accepted format decision.

## Existing provider surface

`@kf/export` exposes `createExport`, `signExportPackage`, `verifyExport` and
`importExport`. `createExport` establishes a PostgreSQL repeatable-read, read-only
snapshot before its reads. It emits ontology plus authoritative database sections,
with a manifest and a database snapshot digest. Format 2 imports require a trusted
manifest signature. The importer validates the package, restores table sections,
checks action provenance and audit-chain consistency, and reconstructs derived
audit state within the caller's transaction.

The section inventory includes Warrant identities, contract revisions, preflights,
dispatches, runtime receipts, submissions, blockers, deviations, discovered gaps,
artifacts, evidence, gate runs, inferences, judgments and resolution requests.
It also carries actions, approvals, snapshots and audit events. These are provider
records; local Git history is not a substitute for them.

`artifacts` and `artifact-versions` preserve database metadata. They do not carry
object-store bytes. Artifact locations and content digests allow reconnection to
be checked, but their presence alone does not prove bytes are available.

Source pointers within that exact provider revision:

- `packages/export/src/internal/exporter.ts`
- `packages/export/src/internal/types.ts`
- `packages/export/src/internal/sections/core.ts`
- `packages/export/src/internal/import-targets.ts`
- `packages/export/src/internal/importer/restore.ts`
- `packages/export/src/extended-roundtrip.test.ts`

## Shared OW-WAR-0111 integration work

Keep the two formats distinct. The OpenWarrant experimental archive is an inert
byte transport and source-reconstruction format. A KF preservation package is a
provider database snapshot with authenticated provenance. Do not translate an
archive's local state strings into fabricated provider actions or approvals.

The integration must:

1. Seed a disposable compatible KF source with actual Warrant operations and
   retained superseded, disputed and annulled records. Preserve the resulting
   provider records, rather than inventing approval evidence in archive metadata.
2. Export through the public provider surface. Retain its exact signed package,
   tool/source identities and all required artifact bytes by content digest.
3. Stop access to the source database and source artifact storage. Restore into
   a separate empty, migrated database with isolated credentials and explicitly
   configured fixture trust keys. Never activate fixture authority in a user repo.
4. Re-export through the public provider surface and compare exact authoritative
   section content and snapshot identity. Separately reconnect every required
   artifact and compare its actual byte digest. Reconstruct the OpenWarrant IR
   from the retained source inputs without consulting the original checkout.
5. Exercise refusals for missing/tampered evidence, untrusted package signatures,
   incompatible format and incomplete required categories. Failed restore must
   leave no committed partial database state.

The provider's extended round-trip test covers several other domains. That does
not establish this Warrant-specific scenario. Existing OW111 local tests prove
bounded transport, IR reconstruction and retained Git bytes only. Complete
provider coverage and the empty-KF round trip remain open. Keep OW111 incomplete
until these observations exist; do not change unavailable categories to absent
to make import pass.

## Later observations

This document's inspection findings above describe the initial boundary, not the
current test result. Provider PR #3 merged at
`876efc7fc1248bc1dab666375e18b634ebbd834e`, adding real source-shutdown database
and object-store restoration. Provider draft PR #4 uses the current source-complete
archive and expanded synthetic runtime receipt records. See
`kf-source-complete-roundtrip.md` for the observed local result and exact archive
identity. Hosted checks for that extension remain separate.

These observations cover provider record preservation and source-detached local
IR reconstruction. They do not supply a local non-human runtime receipt resolver,
real runtime execution proof, accepted format decision or independent disposition.

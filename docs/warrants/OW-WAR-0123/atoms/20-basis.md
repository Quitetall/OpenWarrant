---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ef7-7862-8205-72d16ebcaf9f
role: basis
jurisdiction: authored
order: 20
classification: internal
---


# Basis

## Governing sources

- SAS §20.2: a child references `warrant_ref`, `contract_revision` and
  `contract_digest`. SAS Law 9: a child derives purpose from an exact
  parent revision.
- SAS §91.5 test 29: "Child references exact parent contract revision."
- RQ-023: child WARs cite exact parent revision.
- SAS §28.5–§28.7: contract digests, revision ancestry, no in-place
  amendment.
- SAS Law 15: unknown is not failure and not pass.
- SAS §71.2: `war new` options include `--parent <WAR>`.
- OW-ADR-0021: this Warrant declares `check.rs` (governed by OW-WAR-0114),
  `new.rs` (resolved OW-WAR-0002 and 0062; drafted by OW-WAR-0113) and
  `lib.rs` (OW-WAR-0116). On authorization it becomes their owner.

## What the code does today (read 2026-09-23)

- `crates/openwarrant-core/src/manifest.rs`: `ParentRef` with optional
  `contract_revision` and `contract_digest`; a missing revision is refused
  (`ParentWithoutRevision`).
- `crates/openwarrant-cli/src/check.rs`: `contract_digests` compiles every
  Warrant in the corpus and the `relations.parent-digest` block compares
  each cited digest with the parent's current digest. A missing digest is
  UNKNOWN with the value to paste; a parent outside the repository is
  UNKNOWN.
- `crates/openwarrant-cli/src/contract_history.rs`: `resolve(repo, alias,
  revision)` finds a retained authorized revision in Git history and
  verifies its digest. It refuses a shallow repository.
- `crates/openwarrant-compiler/src/lower.rs` always emits
  `contract_revision: 1`; the revision number lives in
  `authorization.toml`.
- `conformance/plants.d/00-corpus.sh`: "stale parent contract digest"
  edits OW-WAR-0001's intent and expects `relations.parent-digest`.
- The corpus: four children, OW-WAR-0002 to 0005, each citing
  OW-WAR-0001 at `contract_revision = 1` and digest `f29c7d95…`.
  OW-WAR-0001's revision 1 digest is `ab7e2df7…` (its authorization at
  commit `21fad306`); `f29c7d95…` is revision 2.

## Assumptions

- A-001: the parent's authorized revisions are exactly those in its
  current `authorization.toml` and in retained history, as
  `contract_history::resolve` reads them. Confidence: high; it is the
  reader `war diff` already trusts.
- A-002: reading Git history from `war check` for a child that cites an
  older revision keeps `check` deterministic and offline. It reads local
  objects only. Confidence: high.

## Unknowns

- U-001 (**blocking**): how the four existing children are reported. They
  cite revision 1 at revision 2's digest, and their manifests are signed.
  - Option (a): an error for every child. The corpus stays red until the
    owner amends OW-WAR-0003 and 0004. OW-WAR-0002 and 0005 are resolved,
    so their error would never clear.
  - Option (b), recommended: an error when the child is unsigned, where
    the fix is an edit; a warning when the child's contract is signed,
    naming the revision the digest belongs to and the amendment path. The
    signed record stays as it is and the finding stays visible.

  Resolution: the owner chooses before authorization. The obligations
  below are written for (b); under (a), OBL-002's plant changes severity
  and the corpus-wide result in OBL-006 changes with it.
- U-002 (non-blocking): whether a child may cite a parent that has no
  authorized revision (a draft). Today it can, at revision 1 against the
  current compile. This Warrant keeps that behaviour and makes `war new
  --parent` refuse it, so no new citation of a draft is written by the
  tool.

## Residual risks

- R-001: OW-WAR-0113 (draft) also declares `new.rs`. Whichever is
  authorized later owns it; the other rebases. The changes touch
  different flags.
- R-002: history reads are bounded by `contract_history`'s limits (1,024
  snapshots, 32 MiB). A parent past them is UNKNOWN, not an error.

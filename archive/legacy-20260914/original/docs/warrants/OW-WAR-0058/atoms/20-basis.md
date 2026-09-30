---
schema: oh.war/atom/v1
warrant_uuid: 01a06069-882d-7103-a07a-8e2d5c23bd12
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS v0.1.0-draft.1, sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.
- §14 Basis inputs ("SAS revision"); §34.1 stable identifiers; §34.2
  contributions; §34.3 status derived, SAS never edited to tick boxes; §34.4
  architecture-change discovery; §64 format basis; §101 governance of this
  SAS, all six subsections; §106 requirement index.
- RQ-014, RQ-020.

## Measured on 2026-09-02

- The digest `aad5256c…` appears in six places: the SAS front-matter table,
  README, PRODUCTION_ROADMAP, OW-WAR-0001's basis atom, a doc-comment in
  `openwarrant-core/src/lib.rs`, and a comment in `Cargo.toml`. Zero code
  reads it.
- `FormatBasis` has four fields — package, version, root schema, profile
  schema — and no SAS field.
- `AdrRelation::AmendsSas` is declared in `lifecycle.rs` and constructed
  nowhere.
- `traceability::ArchitectureChange` models §34.4's four steps and has no
  caller outside its own test.
- §106 has 57 rows in nine numeric bands and no status column, by §34.3's
  design.
- `docs/sas/` holds exactly one file.

## Measured during execution

- `FormatBasis` is in the CONTRACT digest's preimage, not only the workspace
  basis. Pinning the SAS into it therefore moved every Warrant's contract
  digest as well — and OW-WAR-0001's four children (0002, 0003, 0004, 0005)
  cite it at an exact contract digest (§20.2, RQ-023). Those citations became
  false the moment the parent was bound to a SAS revision, and were re-cited
  to the parent's new digest in the same commit. That is the rule working:
  a child that cites an exact revision must be told when it moves.
- Composition revision digests were unchanged across all 53 Warrants, as the
  work order required.

## Assumptions carried in

- The current document is proposed as `0.1.0-draft.1`, the version its own
  front matter states, with no predecessor and therefore not
  architecture-changing. Acceptance is left to the owner.
- Requirement identity is the canonical id; a retitle is a semantic change
  (§101.3) but not a removal. Whether a retitle should ever be permitted
  without superseding is left to the ADR that accompanies the first one.
- A revision's §106 snapshot is stored in the record rather than
  recomputed from history, so that `war sas diff` needs no second copy of
  the document. The record grows by 57 short lines per revision.

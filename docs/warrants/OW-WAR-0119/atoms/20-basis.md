---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5e99-7663-90dd-3063ec23be35
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS §12.1: every WAR and ADR SHALL have an internal UUIDv7 identity, zero
  or one local alias, zero or one enterprise identifier.
- SAS §12.2: the UUIDv7 is created at draft creation and never changes.
- SAS §12.3: the alias namespace is declared in repository configuration.
- SAS §12.7: machine references SHOULD use UUID or enterprise identity;
  human views may show the alias.
- SAS §91.3 tests 17 and 18: a UUID survives alias allocation; two
  repositories with one alias do not collide.
- SAS §106: WAR-SAS-RQ-001 and WAR-SAS-RQ-002.
- `crates/openwarrant-core/src/identity.rs`, `manifest.rs` (validation),
  `crates/openwarrant-cli/src/journal_cmd.rs` (`journal.wrong-warrant`),
  `relations.rs` (parent and supersedes resolution), `blut.rs`.
- OW-WAR-0114's gap table: RQ-001 and RQ-002 unaddressed, placed at
  `roadmap://OW-PHASE-1/identity`.
- OW-ADR-0021: this Warrant declares `check.rs` and `blut.rs`. On
  authorization it governs the changes it makes to them.

## Assumptions

- A-001: the corpus is clean for the three new errors today. A scan on
  2026-09-23 found 136 Warrant manifests, all UUIDv7, none duplicated, and
  no atom whose `warrant_uuid` differs from its manifest. So the new
  errors fire on no existing Warrant. Confidence: high; the plant's
  positive run re-checks it on the real corpus.
- A-002: `git show HEAD:<path>` is the baseline for `identity.changed`,
  the same one `journal.rewritten` uses. With no HEAD version (a new
  Warrant) the check has nothing to compare and passes. Outside a git
  repository it reports `UNKNOWN`, not pass. Confidence: high.

## Unknowns

- U-001 (non-blocking): the 13 v4 ADR UUIDs. §12.1 wants UUIDv7; §12.2 says
  identity never changes. Both cannot hold for these records. This Warrant
  reports them as warnings and changes nothing. The owner decides whether
  new ADRs with a non-v7 UUID become an error, and whether the existing 13
  are recorded as grandfathered in an ADR.
- U-002 (non-blocking): whether alias-form `war://` in an ADR's
  `governs` should become an error once the six are amended. This Warrant
  keeps it a warning.

## Residual risks

- R-001: OW-WAR-0114 also declares `crates/openwarrant-cli/src/check.rs`
  (D-013). Under OW-ADR-0021 the later authorization governs the path. The
  performer rebases on whichever lands first; the plant is the check that
  both sets of rules survive.
- R-002: a check that compares with HEAD can be defeated by committing the
  change first. `journal.wrong-warrant` and the contract digest still
  catch that case for any Warrant with a journal or an authorization.

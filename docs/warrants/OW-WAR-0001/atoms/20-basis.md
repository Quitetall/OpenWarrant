---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-7f2a-8e39-69730f255e33
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

- `docs/sas/WAR_Software_Architecture_Specification.md`, v0.1.0-draft.1,
  sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.
  Imported byte-identical; the digest was verified against the drafted file.
- SAS §78 (repository structure), §79 (crate responsibilities), §80 (Rust
  stack), §92 (aggregate gate), §98 Phase 1, Appendix E item 1.

## Context

The four-crate v0 layout is taken from §78 verbatim rather than invented. §78
recommends beginning with fewer crates than the eventual ten and splitting "only
at stable authority seams" — so `openwarrant-types`, `openwarrant-schema`,
`openwarrant-source`, `openwarrant-protocol`, `openwarrant-kf`,
`openwarrant-katana`, and `openwarrant-blut` are deliberately not created yet.
Creating them empty would assert seams that have not been shown to be stable.

## Prerequisites

- A Rust toolchain matching the repository pin.
- `cargo-deny`, invoked by the gate.

## Assumptions and Unknowns

- **Evidenced premise.** Rust 1.97.1 is the pinned toolchain and is installed on
  the development host. Verified by running the gate on it.
- **Blocking unknown, deferred not resolved.** Which RFC 8785 implementation to
  use. It is blocking for OW-WAR-0003, not for this Warrant, and it is recorded
  here so it is not mistaken for an oversight.
- **Accepted residual risk.** CI runs on `ubuntu-latest` rather than the
  self-hosted runner the private-repository tier calls for. The runner does not
  exist yet. Accepted because the gate is a single short job, and the exposure is
  bounded by an explicit `timeout-minutes`.

## Constraints and Invariants

- **The Apache-2.0 path must stay open.** This repository ships
  AGPL-3.0-or-later and is intended to become Apache-2.0 when public. Every
  dependency must therefore be MIT and/or Apache-2.0: a copyleft dependency
  adopted now could not be relicensed later. This is enforced by the gate, not
  by review discipline, because a review habit does not survive a busy week.
- **The toolchain pin is exact.** A newer clippy is not a superset of an older
  one. Every gate claim names the toolchain it was made on.
- **No placeholder canonicalization.** A digest minted under a placeholder is a
  digest a later correction silently invalidates.

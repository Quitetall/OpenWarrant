---
schema: oh.war/atom/v1
warrant_uuid: 01a06011-b342-78b3-8ba5-ed5c5cd9ba09
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS v0.1.0-draft.1, sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.
- §17.5 read projections; §34 SAS traceability (34.1 stable identifiers, 34.2
  the five contributions, 34.3 status is *derived*, never assigned); §72.5
  `war status <alias>` is the per-Warrant form, so the corpus form is bare
  `war status`; §98 implementation phases and their Exit sentences; §102
  decision 8; §105 URI forms (`roadmap://<item-id>`); §106 requirement index.
- RQ-022, RQ-074, RQ-075.

## Measured on 2026-08-26

- 57 requirement IDs in §106, banded (001–005, 010–015, …, 080–084). A counter
  cannot index by number.
- 51 referenced by at least one manifest; 48 with a `complete` claimer; 3
  partial-only; **6 unaddressed**: RQ-001, -002, -021, -023, -060, -065. Several
  of those are demonstrably built. The gap is bookkeeping, and the projection
  surfaces it rather than hiding it.
- `traceability.rs` models the whole §34.3 ladder — `RequirementRef`,
  `Contribution`, `RequirementStatus`, `derive_status`, `derive_all` — and
  has zero callers.
- `roadmap://OW-PHASE-N/<slug>` is an unparsed `String` (`manifest.rs:158`).
  49 refs, 49 distinct slugs — they are per-Warrant labels, not a grouping. The
  one principled slug is `exit`: every phase 0–8 has exactly one, and it is the
  Warrant that discharges §98's Exit criterion.
- Phases 9 and 10 have no §98 Exit sentence.
- `PRODUCTION_ROADMAP.md` says "resolved" 48 times. **Zero resolution records
  exist on disk**, and `war resolve` refuses to write one. The projection reads
  records only and never that column.
- Every Warrant's §24 state is `Provenance::Derived` and reads `draft`; the
  journal (OW-WAR-0031) is not landed.

## Assumptions carried in

- The roadmap grammar is an OpenWarrant convention. §105 says only
  `roadmap://<item-id>`; the SAS's own example uses a milestone id
  (`roadmap://LIM-PHASE-1/M4`). The parser accepts a slug and does not require
  it to be a milestone id. Recorded as a residual risk.
- "Objective achieved" is defined as the `exit` Warrant resolving satisfied.
  The Exit sentence itself is printed verbatim and never evaluated — no tool can
  judge "Liminal is the single production document semantic compiler".
- The Release axis has exactly one member until OW-WAR-0058 lands. The
  projection carries `sas_revision: null` rather than inventing one.

## Alias note

This Warrant's alias was set by hand to `OW-WAR-0055`, above the 0051–0054
range in flight on another branch, after `war new` allocated `0051` against a
branch that did not yet contain them. The UUID is the one `war new` generated;
per RQ-001 and RQ-002 the UUID is the identity and the alias is a local label,
so a later collision on the alias is a directory conflict at merge — loud — and
never two records with one identity.

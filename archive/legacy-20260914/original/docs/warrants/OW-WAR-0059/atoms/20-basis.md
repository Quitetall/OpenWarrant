---
schema: oh.war/atom/v1
warrant_uuid: 01a060c6-64a5-71e1-b994-133d3c1e19d2
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS v0.1.0-draft.1, sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.
- §27.2 (an agent SHALL NOT resolve a delivery); §38.6; §44.5 required
  passing result; §44.6 gate receipt, all fifteen items; §45 gate
  invalidation ("no historical evidence is rewritten"); §51.3 a performer's
  claim is not evidence; §56.1 thirteen requirements; §56.2 record; §56.3
  falsification; §56.4 dispute; §56.5 annulment.
- RQ-053, RQ-054, RQ-059.

## Measured on 2026-09-02

- 38 Warrants with exactly one §56.1 requirement unmet; the same one on
  all 38. Three of them (OW-WAR-0010, 0016, 0020) with every declared
  obligation established by an admissible verification, so §38.6 would
  read `satisfied`; 35 with obligations not established.
- `GateReceipt` implemented with all §44.6 scalars and minted by `war gate
  --run --record` into the gitignored receipts path; `receipt_digest`
  computed with the field empty, over the rest.
- `Resolution` implemented with §56.2's fields and `validate`; constructed
  by no code outside its tests. `DigestDomain::AssuranceCaseSnapshot`
  declared, computed by nothing.
- `war resolve` without `--dry-run`: a hard-coded refusal citing an authority
  model that has existed since OW-WAR-0055's seam.
- The corpus projection's first caveat: "Gate runs are NOT read."

## Measured during execution

- The `.gitignore` comment on `docs/receipts/` already said what this
  Warrant does — "a receipt becomes committed evidence deliberately, as
  part of a resolution (OW-WAR-0046 deliverable 3)" — and no code did it.
- Copying a receipt into a Warrant and rewriting its stream refs to the new
  location breaks the seal, correctly. The receipt has to be MINTED into the
  tracked directory, which meant threading an output directory through the
  runner rather than adding a copy step.
- OW-WAR-0016 has every obligation established and cites no gate. `war
  evidence record` refuses it, correctly: requirement 5 is unmet for it
  because nothing was asked, and a receipt cannot answer a question the
  contract never posed. Two Warrants, not three, reach all thirteen here.
- The gate this repository cites is `war check --generated`, and a freshly
  minted receipt changes the projection it checks. Recording evidence for
  one Warrant therefore makes the next Warrant's run fail on
  `corpus-status.drift` until `war compile` runs between them. The order
  is: compile, record, compile, record. Both receipts here were minted
  against a fresh projection; the run's own stdout shows it.
- The end-to-end authorization test copied the real corpus and assumed
  OW-WAR-0014 had no records; it now removes them first. Recording real
  evidence and real resolutions into the corpus means every test that
  copies it must say which state it wants.

## Assumptions carried in

- Binding a receipt to the contract digest, not the workspace basis digest,
  is the right strictness: an edit to a non-contract atom does not
  invalidate a run, an edit to the contract does. §45's invalidation is
  about the gate side; this is the subject side.
- `artifact_manifest_digest` is the sha256 of `deliverables.toml`'s bytes.
  §56.2 names the field and nothing defines the manifest's canonical form;
  the bytes are what the resolver could read.
- `profile_outcome` is free text from the resolver (§56.2's example is
  `delivered`); the delivery profile defines no vocabulary for it.

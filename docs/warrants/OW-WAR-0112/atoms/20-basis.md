---
schema: oh.war/atom/v1
warrant_uuid: 01a0ca4a-0c02-7cd3-b49d-786377a1aa06
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS §21.2 and §21.5 (supersession: currency and what is adopted), §27.2
  (human-only acts), §28.4 and §28.7 (what an authorization records; never
  patched), §31 (material amendment), §34.4 (discovering a wrong
  requirement: ADR, revision, amend, preserve), §37 (deliverables) and the
  new §37.5, §44.8 (a mutating gate is a different thing), §56.1 requirement
  3 and §56.2 (what a resolution binds and records), §76.2 (likely
  remediation), §101.3 (an architecture-changing revision carries an ADR).
- OW-ADR-0012 (the correction act, which this narrows and does not
  replace), OW-ADR-0016 (a Warrant keeps its Basis until an amendment
  re-pins it), OW-ADR-0019 (a rendering issues commands and holds no key;
  the SAS 1.1.0 text it drafts becomes 1.2.0 by OW-WAR-0071 AM-001),
  OW-ADR-0020 (ratatui, confined to `tui/`, synchronous), OW-ADR-0021 (this
  Warrant's decision).
- OW-WAR-0073 (superseded here; its work order is adopted verbatim as
  deliverables 1-8 of M2), OW-WAR-0069 (the board, the hotline, the console;
  `sign.preset` and `console::board` are its seams), OW-WAR-0072 (the batch
  act, authorized and unimplemented — the re-pin's single signature waits on
  it), OW-WAR-0064 (corrections), OW-WAR-0063 (`traceability.unknown-requirement`
  reads §106 from the document text, which is why RQ-037 resolves before the
  revision is accepted).
- `docs/design/openwarrant-friction.md` Q2, the owner's answer of 2026-09-13:
  preserve the historical delivered version and its evidence; let the new
  Warrant govern the new version, with explicit lineage.
- `docs/design/friction-findings.md`, the measured record from which this
  Warrant's problem statement is taken.

## Assumptions carried in

- The declaration set — sorted `(id, target_ref)` pairs — is the thing an
  authorizer grants, not the bytes. `war pins --refresh` rewrites content
  digests during ordinary work, so a whole-file digest of
  `deliverables.toml` would be invalidated by the first refresh. Recorded as
  the reason OW-ADR-0021 Decision 1 is shaped as it is.
- Marking OW-WAR-0073 `currency = "superseded"` does not move its signed
  contract digest: `current_coverage()` names eight elements and `currency`
  is not among them, nor is `manifest.toml` an attestation subject. Verified
  in `crates/openwarrant-compiler/src/ir.rs` on 2026-09-22 before the mark
  was written.
- `notify` is not a dependency of this workspace, contrary to 0073's
  deliverable 5; live refresh reuses `watch.rs`'s fingerprint poller.
  Behaviour delivered, mechanism differs, stated here rather than in a
  journal 0073 no longer accrues.
- `Diagnostic` has no `info` severity. "Superseded by a later owner" is a
  `pass` with a message, the shape `deliverable.corrected` already uses.
- The authority files may be written by the tool from a human's answers at
  a terminal, once — the owner's decision of 2026-09-22, which changes the
  rule stated in `roles.toml.example`, `allowed_signers.example`,
  `docs/RESOLVING.md` and `QUICKSTART.md`. All four move in M4's commit. The
  gate is `at_a_terminal()`, the one `war sign` already leans on, and its
  residual is THREAT_MODEL entry 2's, accepted again in the same words.
- Ownership orders by a locally stamped effective time. Two authorizations
  signed out of order would flip an owner; ingestion refuses an
  authorization whose time precedes an existing owner of the same path.
  Recorded in OW-ADR-0021's consequences as the residual it is.

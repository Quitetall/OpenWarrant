---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32d9-76e0-8417-2d69773249ca
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS 1.1.0 §27.6 (the batch act), §27.2 (human-only acts, unchanged),
  §85 (attestation of an ssh-signed act), §106's three new rows — all
  delivered by OW-WAR-0071 and **not yet accepted**.
- `OW-ADR-0019` (the decision), `OW-ADR-0015` (the DSSE envelope),
  `OW-ADR-0012` (the correction act, whose two-half seam this copies).
- `crates/openwarrant-cli/src/sign.rs` as it stands: `Pending`,
  `sign::pending`, the per-act ingest paths, `ssh_sign_file`.
- `docs/THREAT_MODEL.md` entries 1–3 and 10 (replay across namespaces).

## Assumptions carried in

- SAS 1.1.0 is accepted before this Warrant is authorized. If the owner
  declines the revision, this Warrant is withdrawn, not amended.
- The batch document is a new record type, not a change to any existing one:
  authorization, resolution, correction and SAS acceptance records keep their
  schemas and gain one optional `batch_ref` field, which is additive under
  `docs/COMPATIBILITY.md`.
- `war console`'s checked set is the natural source of a batch, so this
  Warrant touches `console.rs` only to pass the set to the new seam.

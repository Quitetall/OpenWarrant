---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32d7-7473-b6e1-fbef061a1e5f
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS 1.0.0 as accepted (`docs/sas/revisions/1.0.0.toml`, sha256
  b7105f52…): §27.2 (human-only acts), §34.4 (supersede, never erase),
  §38 (resolution), §56.1 (the thirteen requirements), §44.6 (receipts),
  §101.2 and §101.3 (revisions, and the ADR an architecture-changing one
  requires), §106 (the conformance rows).
- `docs/THREAT_MODEL.md` entries 1–3: `ssh-add -c` is the control that makes
  a signature the human's, and `war` cannot verify it was used.
- `OW-ADR-0015` (attestations): every ssh-signed act leaves a DSSE envelope,
  so a batch act must say what its envelopes' subjects are.
- `OW-ADR-0016` (SAS 1.0.0) as the predecessor decision.
- `docs/PROJECTION_CONTRACT.md`: what a projection may claim.

## Assumptions carried in

- The owner asked for batch signing before a dashboard on 2026-09-12,
  choosing it over dropping `-c`. Recorded in the rationale as the
  blocking unknown that only their signature resolves.
- 1.1.0, not 1.0.1: this adds requirements rather than correcting text, and
  §101.2's rule is that a revision adding normative rules is not a patch.
- No Warrant authorized against 1.0.0 is re-pinned by this revision. Each
  keeps its Basis until an amendment carrying `sas_revision` re-pins it and
  a human re-authorizes (OW-ADR-0016's rule, unchanged).

## Out of scope

The implementation seams. This Warrant delivers text, a revision record and
an ADR; nothing in `crates/` moves under it.

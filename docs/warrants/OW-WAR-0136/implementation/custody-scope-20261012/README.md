# Custody and invalidation scope audit

All 30 selected runtime controls and four invalidation unit tests pass on Rust 1.97.1. The runtime tally includes the positive corpus check. The controls exercise attestation subjects, resealed replacement receipts, all nine custody fields, performer refusals, signed and unsigned invalidation, transitive disputes, unaffected resolutions, duplicate invalidation, dispute tampering, inadmissible receipts and historical byte preservation. Scratch keys and verifier fixtures exercise protocol behavior; they do not establish real owner or independent qualification. Binary identity is retained with its earlier dirty build basis, rather than misrepresented as a clean final-commit build.

A real read-only custody query on resolved OW14 returns intact while each receipt's original digest remains UNKNOWN (`not attested`). This verifies the retained historical limit; an intact current seal is not proof of original custody. A first query on OW1 was refused `attest.custody-unresolved` because that Warrant has no resolution; that refusal is retained separately. No custody record or signature was written.

The resolution dry run exits 2: five PASS, four WARN and nine UNKNOWN. Delivery paths D-004, D-006, D-011 and D-012 are now governed by later OW148 work; the records are not repinned. Exact authorization, digests, dispositions, gate results, unknowns, blockers, judgments, independence and risk authority remain unmet.

OBL-006's existing independent `not_established` finding is preserved. The demonstration gate's existence does not prove a human-signed resolution, distinct-auditor custody record, real owner invalidation and the resulting single dispute. The Basis also retains Q-001/Q-002 as blocking choices; this audit does not infer an owner answer from implemented code. OW136 stays in progress pending the full registered gate, delivery reconciliation, independent assessment and the required owner acts.

All retained repository attestations also verify: 268 PASS, zero WARN, UNKNOWN or ERROR. This checks existing signatures and their attested subjects, not the unperformed owner demonstration or original digests that were never attested.

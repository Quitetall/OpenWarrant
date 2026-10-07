---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-604b-7d63-8453-39847a16c82c
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§98 Phase 9's exit: "a resolution is signed, its evidence custody is
audited, and one gate invalidation propagates to every dependent
resolution, with no step performed by the actor who produced the work."
No Warrant owns it (OW-WAR-0114's gap table: "Phase 9 has no exit
Warrant"). Measured against the code on 2026-09-23:

| clause | state |
|---|---|
| a resolution is signed | **exists.** All 29 recorded resolutions carry `attestations/resolve-1.dsse.json`; `war attest --verify` checks the signature and subject digests. `may_resolve` refuses the performer. |
| its evidence custody is audited | **missing.** The resolve attestation's subjects are `resolution.toml`, the signed response and the contract digest. The receipts the resolution relied on are named by path in `gate_run_refs`, with no digest, and are not attestation subjects. A receipt replaced after resolution by another that reseals is not detected. §41.5's custody fields are not recorded or reported anywhere (`rg -i custody crates` finds only `doctor.rs`'s signer custody). |
| one gate invalidation propagates to every dependent resolution | **library only.** `gate_run.rs::propagate_invalidation` computes the transitive set and is unit-tested. Nothing in the CLI calls it. There is no invalidation record, no way to invalidate a gate, no dispute record (`resolution.rs::dispute` returns a value nothing persists), and no plant for §91.10 test 75. OW-WAR-0020 claimed RQ-057 complete on the function. |
| no step by the actor who produced the work | **partial.** Resolution and authorization refuse the performer. Invalidation and audit do not exist, so nothing refuses the performer there. |

All 29 resolutions in this repository rest on
`gate://software.repo.war-check@1.0.0`. Invalidating it for real would
dispute all 29 at once.

## Desired Outcome

- **Custody is audited.** A resolve attestation names each relied-on
  receipt (and its run, stdout and stderr) by digest.
  `war attest --custody <alias>` reports, per receipt, each §41.5 field as
  present or `UNKNOWN`, and fails on any subject that moved since
  signing. The audit is recorded by an actor who is not the performer.
- **Invalidation propagates.** Invalidating a Gate Definition version is
  an act by an actor who is not the performer of any Warrant it reaches.
  Its ingest runs `propagate_invalidation` over the corpus and writes one
  §56.4 dispute per dependent resolution, transitively. No resolution file
  is edited; standing is read from the disputes (§45 clause 4, "no
  historical evidence is rewritten").
- **Standing shows it.** `war check` and `war status` report a disputed
  resolution as disputed. A receipt from an invalidated gate is not
  admissible for requirement 5 on any unresolved Warrant.
- **The exit is demonstrated** on a scratch corpus by plant, and in this
  repository if Q-002 selects it.

## Non-goals

- Closing a dispute (§45 clause 6: re-verification on a new gate version,
  then resolve the dispute or annul). The exit asks for propagation; the
  closing act is a later Warrant.
- Knowledge Fabric as the invalidation registry (§45 clause 1). This is
  the local stand-in, recorded under `docs/gates/`, until OW-WAR-0028
  federates.
- Physical test profile, independent human workflow, regulatory mapping,
  audit checkpoints: Phase 9 "as required" items that the exit sentence
  does not name.
- Invalidating `software.repo.war-check@1.0.0` in this repository.

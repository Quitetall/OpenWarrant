---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-604b-7d63-8453-39847a16c82c
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- §98 Phase 9, its exit sentence (quoted in Intent).
- RQ-057 and §45: when a Gate Definition is invalidated, dependent
  bindings and resolutions are located; materially dependent resolutions
  become disputed according to policy; historical gate runs remain
  preserved; no historical evidence is rewritten.
- §91.10 test 75: "Invalidated gate disputes dependent resolution."
- §56.4: a dispute identifies the challenged resolution, grounds, affected
  evidence or judgment, reliance policy, owner, and required
  re-verification.
- §41.5: high-assurance evidence SHOULD record collector, original digest,
  transfer method, storage event, instrument or runner identity,
  calibration or qualification, transformations, access history, and
  derivative lineage.
- RQ-059: a resolution binds its exact contract and assurance snapshot.
- RQ-053 and §27.2, §27.3 condition 4: the performer and the resolver are
  distinct; a performer's report cannot satisfy an independent gate.
- §43.3: Gate Definitions are immutable; `invalidated` is a lifecycle
  state. `gate.rs` already refuses to bind an invalidated definition.
- OW-ADR-0015 (attestations), OW-ADR-0021 (ownership).
- Code read for this draft: `core/gate_run.rs` (`propagate_invalidation`,
  `DependentResolution`), `core/resolution.rs` (`dispute`, `annul`),
  `core/authority.rs` (`ActorRole`, `may_resolve`), `cli/sign.rs`
  (`attest_after`: the resolve attestation's subjects), `cli/attest.rs`,
  `cli/evidence.rs`, `cli/resolution_cmd.rs` (`check`).
- Existing Phase 6 and 9 Warrants: OW-WAR-0020 (gate runs and §45,
  resolved), OW-WAR-0022 (resolution, dispute, annulment, resolved),
  OW-WAR-0046 (Phase 6 exit, resolved; its non-goals send "signatures and
  custody" to Phase 9), OW-WAR-0067/0071/0072/0073
  (`roadmap://OW-PHASE-9/release`). None owns custody, a persisted
  invalidation or a dispute record.

## Assumptions

- A-001: "the actor who produced the work" is the performer recorded for
  each Warrant the act reaches (`repo.performer()`). Every new act
  refuses that actor, by the same `SelfAct` refusal `may_resolve` uses.
  Confidence: high.
- A-002: a resolution "materially rests on" another when its Warrant names
  the other as a parent (§20.2) and the other is resolved. That is the only
  Warrant-to-Warrant dependency the corpus records. Confidence: medium; a
  resolution citing another's receipt directly would also count, and none
  does today.
- A-003: the custody audit is a check, not an authority act. The blind
  verifier (OW-WAR-0117), or any actor that is not the performer, may run
  and record it. Confidence: medium; if the owner wants it human, it
  becomes a signed act and Q-001's answer covers it.
- A-004: receipts relied on by the 29 existing resolutions are not
  attestation subjects and cannot be made so without re-signing. Their
  custody audit reports `original digest: UNKNOWN (not attested)` and
  compares against the receipt's own seal only. Confidence: high.

## Unknowns

- **Blocking unknown — Q-001: who may invalidate a gate, and is it a signed
  act?** Options:
  - (a) a new human act, `invalidate`, signed with `war sign` by a holder of
    `resolver`, attested like the other four. A fifth human act: AGENTS.md,
    SIGNING.md and the SAS's act list change.
  - (b) a new role, `gate_steward`, in `roles.toml`, with the same signed
    act. The same change, plus a role.
  - (c) any registered actor that is not the performer, agents included,
    unsigned: invalidation is a finding, and dispute is "according to
    policy".

  Recommendation: (a). An invalidation disputes resolutions a human
  signed, so a human signs it. The resolver already owns standing.
- **Blocking unknown — Q-002: where the exit is demonstrated.** Options:
  - (a) on a scratch corpus only, by plant (`58-invalidation.sh`);
  - (b) the plant, plus this repository: a demonstration gate
    (`ops.exit-demo@1.0.0`), one small Warrant resolved against it by a
    human, and then that gate invalidated for real;
  - (c) invalidate `software.repo.war-check@1.0.0` here and dispute all 29
    resolutions.

  Recommendation: (b). The exit says "a resolution is signed", which a
  scratch corpus can plant but cannot sign with the owner's key. (c)
  disputes 29 correct resolutions to prove a point.
- U-003 (non-blocking): dispute closure. Out of scope (Intent). Disputes
  written here stay open until a later Warrant closes them.

## Dependencies

- OW-WAR-0117 (the verifier): requirement 10 for this Warrant's own
  resolution, and A-003's auditor.
- OW-WAR-0133 also declares `evidence.rs`. Sequence 0133 first; this
  Warrant's change there is one rule (an invalidated gate's receipt is not
  admissible).

## Residual risks

- R-001: `propagate_invalidation` is O(n²) on a long dependency chain
  (its own comment). Fine at this corpus's 29 resolutions; not measured
  beyond.
- R-002: under Q-001 (c), an agent could dispute every resolution in the
  corpus. Disputes do not annul, but they do block reliance.
- R-003: the 29 existing resolutions keep an unattested custody chain
  (A-004). The audit says so for each; it cannot repair it.

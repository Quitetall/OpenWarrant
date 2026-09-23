---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f6e-7d90-8eed-cf0953b77657
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS §81 (canonical compiler interface, illustrative), §81.1 (request
  inputs), §81.2 (result fields).
- SAS §83.1 (KF SHOULD invoke a pinned binary through canonical JSON),
  §83.2 (runtime pins, each a SHOULD), §83.3 (sandbox), §83.5 (KF stores
  normalized fields and immutable canonical snapshots).
- SAS §65 (digest domains), §69 (protocol versioning), §75.2 (a process
  seam: request and result are documents).
- RQ-081 (cross-system digests name algorithm and domain), RQ-020 (a
  normative decision is an ADR).
- `docs/design/openwarrant-product-spec.md`, the Phase 3 row and "The
  intended composition": the interface is named and left to be designed.
- OpenWarrant code as it is: `crates/openwarrant-cli/src/compile.rs`
  (`war compile`), `crates/openwarrant-cli/src/sdk.rs` (`war sdk`, its
  input and output limits, its operations), `crates/openwarrant-compiler`
  (canonical IR, digests).
- Knowledge Fabric, read at `/mnt/4tb/openhuman-knowledge-fabric` commit
  `3d3c871e`:
  - `docs/decisions/0019-warrants-as-institutional-record.md`: KF records
    what OpenWarrant computed and does not recompute; authored atoms never
    enter its database;
  - `docs/decisions/0002-liminal-backed-document-compiler.md` and
    `0010-liminal-compiler-deferred.md`: KF's compiler runtime is a
    document compiler speaking `kf-document-v1`; v1.0 carries none;
  - `apps/worker/src/compiler-runtime/`: the runtime and its input guard.
    It starts no OpenWarrant process.
- OW-WAR-0110 (TypeScript types for KF), OW-WAR-0044 (Phase 4 exit),
  OW-WAR-0114 (named this gap, `roadmap://OW-PHASE-4/kf-compiler`).

## Assumptions

- A-001: the process seam of §83.1 is the default shape, because it is
  what KF already does for Liminal (a pinned process, a versioned
  protocol, documents in and out). Confidence: medium. Q-001 can overturn
  it.
- A-002: the interface can be written before either side builds it, as
  long as every element is marked as existing or to build. Confidence:
  high.

## Unknowns

- U-001 (blocking): who compiles. The product spec says KF supplies inputs
  to OpenWarrant's compiler. KF ADR 0019 says the repository side computes
  and KF records. The interface is different in each case.
  *Resolution requirement:* the owner and KF's owner answer Q-001.
- U-002 (blocking): which component is "the Knowledge Fabric Compiler".
  Nothing in either repository carries that name as code.
  *Resolution requirement:* KF's owner answers Q-002.
- U-003 (blocking if Q-001 is (b) or (c)): KF would hold authored atom
  bytes to supply them, which KF ADR 0019 says never enter its database.
  *Resolution requirement:* KF's owner answers Q-003.
- U-004 (non-blocking): protocol names. The ADR proposes
  `oh.war/compilation-request/v1` and `oh.war/compilation-result/v1`;
  the owner may rename them.

## Questions for the owners

- Q-001 (U-001): who runs the OpenWarrant compiler?
  (a) the repository side. KF receives the result inside §67 action
  payloads, as KF ADR 0019 records today; the "interface" is then those
  payloads;
  (b) KF invokes a pinned OpenWarrant binary over bytes it fetched from the
  Source Holder at a named commit (§83.1, the product spec);
  (c) both: the repository compiles, and KF recompiles to check the digest
  it was sent.
- Q-002 (U-002): what is the Knowledge Fabric Compiler?
  (a) KF's existing compiler runtime, with an OpenWarrant adapter beside
  the Liminal one;
  (b) a new KF component;
  (c) a workflow outside KF that calls both.
- Q-003 (U-003): may KF hold authored atom bytes to compile them?
  (a) transiently, for one compilation, never stored;
  (b) stored, with Git still the Source Holder;
  (c) no. Then only Q-001 (a) is open.

## Residual risks

- R-001: the interface is accepted and then one side builds something
  else. Each element names its owner and status, so the gap shows up in
  review, not in production.
- R-002: §81 is illustrative. A later SAS revision could fix a different
  shape. The ADR is then superseded, not edited.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6068-7151-8462-b9ce53fa3227
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the seam changes nothing for the existing profiles
- **scope:** `role.rs`, `manifest.rs` and `profiles/{delivery,decision}.toml`,
  over this repository's corpus.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - every Warrant in the corpus compiles to the same contract and
    composition digests before and after M1. If U-003 moves the pack, the
    move is recorded once and named;
  - `war check --generated` reports no drift attributable to the seam;
  - `Profile::from_str("experiment")` is still refused.

### OBL-002 — a contractor Warrant missing a contractor role fails closed; elsewhere nothing loosens
- **scope:** manifest validation over fixtures in
  `conformance/fixtures/contractor/`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a contractor manifest without `contractor.acceptance` is refused, and
    the error names the profile and the role;
  - a `delivery` manifest declaring a required `contractor.terms` is still
    refused `UnknownRequiredRole`;
  - an unknown profile name is refused;
  - an optional `contractor.terms` on a `delivery` Warrant is preserved,
    as before.

### OBL-003 — the technical core is unchanged, and the terms live only in the profile
- **scope:** the fixture compiled twice, as `contractor` and as
  `delivery` with the contractor atoms removed; the frozen-module list in
  the work order.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the two IRs are equal after removing `identity.profile`,
    `format_basis.profile_schema_id`, the `contractor.*` composition
    entries and the digests that cover them;
  - `git diff <seam-commit>..HEAD` over the frozen modules is empty;
  - a plant that adds an `invoice` field to a core struct fails this
    check;
  - no compensation, party or legal term string from the fixture appears
    in any core IR field.

### OBL-004 — contractor acceptance is the existing human resolution act
- **scope:** the acceptance-authority check, on the fixture program.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - acceptance authority naming an agent is refused by kind;
  - naming an actor without `resolver` is refused by name;
  - naming the performer is refused (§27.2);
  - no new `ActorRole` variant exists (source check);
  - a `kf://` invoice reference reports UNKNOWN, not PASS and not ERROR.

### OBL-005 — the Phase 10 Exit, bounded by U-001
- **scope:** `roadmap://OW-PHASE-10/exit`, over the fixture only.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - OBL-003's result, and the fixture resolves through the existing act;
  - the legal, finance and QMS decisions §98 requires are cited by
    reference.

  Without those citations this obligation is not established, whatever
  the fixture shows. Under U-001 option B it moves to a later Warrant.

## Gate Adequacy

Required at `controlled`: the profile names acceptance authority and legal
terms. The load-bearing obligation is OBL-003. A contractor profile that
reached into the core would make every technical Warrant carry commercial
semantics, which is what §2.2 and §4.4 forbid.

**Adversarial question:** could a contractor atom change what counts as
resolved? Only through the core. The frozen-module diff and the IR
comparison are the two checks that it did not. Acceptance still goes
through `resolve`, which refuses an agent by kind whatever
`contractor.acceptance` says.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6068-7151-8462-b9ce53fa3227
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- **§2.2.** The architecture SHALL support additional profiles, contractor
  work among them. The core identity, composition, authority, evidence and
  resolution model SHALL NOT require redesign when they are added.
- **§4.4.** A future contractor-work profile uses the same technical
  composition without redefining the core protocol.
- **§16.3, §16.4.** Required roles by profile. "Contractor terms" is a named
  future extension role. Unknown required roles fail closed; unknown
  optional namespaced roles are preserved.
- **§22.3.** The contractor profile's field list, and the rule "until that
  profile is approved, a technical WAR SHALL link to, not replace, the
  Knowledge Fabric contractual Work Order and finance records".
- **§98 Phase 10.** Deliverables, the precondition (separate legal, finance
  and QMS decisions) and the Exit.
- **RQ-013** (typed, ordered, deterministic composition) and **RQ-015**
  (required atom omission fails closed), for the profile's required roles.
- **Code read on 2026-09-23:** `role.rs` `Profile`, `manifest.rs`
  `validate` and `UnknownRequiredRole`, `lower.rs` identity and
  format basis, `new.rs` `template_atoms`.

## Assumptions

- A-001: expressing contractor atoms as namespaced extension roles keeps
  them out of every core type. The IR carries them as composition entries
  with digests, and no core struct gains a field. Confidence: high.
- A-002: contractor acceptance maps onto the existing resolution act, with
  the acceptance authority named in `contractor.acceptance` and checked
  against `roles.toml`'s `resolver`. Confidence: medium. A customer who is
  not in the register cannot accept until added by a human, and that is
  probably right.
- A-003: the registry's definitions are pinned like the schema pack. A
  profile definition changed after a Warrant compiled against it moves that
  Warrant's workspace digest. Confidence: medium (U-003).

## Unknowns

- **U-001 (blocking): the legal, finance and QMS decisions §98 requires
  first.** Options:
  - **A.** Wait. This Warrant stays unauthorized until the three decisions
    exist as records it can cite.
  - **B.** Split. Authorize the technical mechanism now, with a fixture
    marked non-binding. Leave `OW-PHASE-10/exit` for a later Warrant that
    cites the decisions. This Warrant then drops its `exit` roadmap ref
    before authorization.
  - **C.** The owner records that no external decision is needed for a
    technical fixture, and says why, as an ADR. This Warrant then carries
    the Exit as drafted.

  Recommended: B. It keeps §98's precondition true for the part that needs
  it.
- **U-002 (blocking for M1): registry or variant.**
  - **A.** A data-driven registry (`profiles/<name>.toml`, pinned), with
    `Profile` becoming a validated name. (Recommended: one core change,
    after which §2.2 holds.)
  - **B.** Add `Profile::Contractor` to the enum. This changes the core
    for every future profile, which §2.2 forbids needing.
- U-003 (non-blocking): whether profile definitions enter the schema pack.
  If they do, the pack version moves once (OW-WAR-0032, OW-WAR-0114's
  precedent). Escalate before M1 lands it.
- U-004 (non-blocking): with no Knowledge Fabric locally, a `kf://`
  reference in `contractor.commercial` cannot resolve. `war check`
  reports it UNKNOWN, not PASS and not ERROR (Law 15).

## Residual risks

- R-001: a profile that requires legal terms by reference cannot check the
  terms' content. The record proves the reference was made, not that the
  contract says what the atom claims.
- R-002: the "unchanged core" claim is bounded by the frozen-module list.
  A change to a module off the list would not be caught by that plant.

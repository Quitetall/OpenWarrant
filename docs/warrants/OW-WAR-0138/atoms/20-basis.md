---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-605c-73a0-af03-c3ad0e0efc55
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- **§27.2.** No self-authority. An agent that can write the register can
  grant itself what §27.2 withholds.
- **§27.3.** Policy-service resolution needs policy to "explicitly allow"
  it. Where that policy lives decides who can allow it.
- **RQ-044.** Agent authority is explicit and bounded. Only if the
  boundary's files sit outside the agent's reach is it more than a
  convention.
- **OW-WAR-0096** (authorized, implementation in progress).
  - It supplies `authority_transition.rs` and `war authority
    draft/propose/approve/activate/allows`.
  - Its notes list what is left: "route privileged consumers through
    authenticated actor and current-state checks; disable legacy fallback".
  - This Warrant is that routing for `war sign` and `authority_check`,
    and nothing else from 0096's list. It does not deploy the store.
- **THREAT_MODEL rows 1, 2, 6 and 11,** and the candidate-transitions table.
- **Product spec, "Engineering contracts still to specify":** "Human/session
  authentication, protected policy state and credentials, actor roles, local
  authority". Also: "ordinary interactive approval occurs within an
  authenticated, unlocked session; policy can require stronger user-presence
  confirmation."

## Assumptions

- A-001: an OpenSSH signature made by an `sk-` key carries the
  authenticator's flags (user presence, user verification) and a counter.
  The flags are covered by the signature (PROTOCOL.u2f, PROTOCOL.sshsig).
  Confidence: medium.
  - Whether `ssh-keygen -Y verify` enforces or prints them is not known
    (U-002).
  - The fallback is for `war` to parse the verified signature blob.
- A-002: 0096's `Revision` has no policy field and `deny_unknown_fields`.
  Adding policy is a new revision schema version, never an edit of v1.
  Confidence: high (read 2026-09-23).
- A-003: 0096's `Principal` carries roles but no actor kind. Human versus
  agent must be carried in the revision too, or the store cannot answer
  §27.2's refusal by kind. Confidence: high.

## Unknowns

- **U-001 (blocking): how a repository adopts protected state.** Options:
  - **A.** Opt-in per repository, and `war check` warns
    `authority.unprotected` everywhere else. When a store is configured it
    governs, with no fallback. (Recommended: today's one-person setup is
    unchanged, and the setup target of ten minutes holds.)
  - **B.** Required for `controlled` and `high` Warrants, and a
    `controlled` Warrant cannot be authorized without a store.
  - **C.** No separate store. Sign `roles.toml` itself through 0096
    transitions and pin its digest.

  B protects the most and costs a second OS account. C protects the
  register's bytes but not policy or presence.
- U-002 (non-blocking, first step of M2): whether `ssh-keygen -Y verify`
  (OpenSSH 10.5 here) refuses or reports an `sk` signature without
  user presence. If it does neither, `war` reads the flags from the
  signature blob after `ssh-keygen` accepts it.
- U-003 (non-blocking): which policy keys are protected. The draft names
  four; M3 lists them in the contract document, and adding one later is a
  store revision.

## Residual risks

- R-001: presence is not identity. A touch proves someone touched the key,
  not who. Key custody stays the operator's (THREAT_MODEL "out of scope").
- R-002: a repository without a store is exactly as protected as today.
  The difference is that `war check` says so.
- R-003: the store is only as protected as the account holding it (0096's
  table). `war` checks the mode and the owner it can see. It cannot prove
  the deployment.

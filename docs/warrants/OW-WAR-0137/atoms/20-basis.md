---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6056-70f1-a94b-56ec20257a39
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- **§27.2.** An agent never authorizes, resolves or accepts residual risk.
  Assignment cannot change that.
- **§27.4.** The system records the role actually exercised. Human views do
  not claim four-eyes review when none occurred.
- **§46, RQ-053.** Independent verification. A performer's report cannot
  satisfy an independent gate.
- **OW-ADR-0021, THREAT_MODEL row 12.** The pattern this Warrant copies: the
  request lists a set and its digest, the signed response echoes it, and a
  set moved after signing is refused.
- **Roadmap.** `OW-PHASE-9/teams`, placed by OW-WAR-0114's gap table
  ("Teams: review assignment, a multi-person queue", product spec).
- **Code read on 2026-09-23.**
  - `sign.rs` `eligible` returns the request's eligible list;
    `choose_actor` refuses an ambiguous one.
  - `verify.rs` `ingest` and `verification.rs` `admissible_for` check
    only that the verifier is non-empty, distinct from the performer and
    independent enough for the assurance level.
  - `authority.rs` already has the `Verifier` actor role. Nothing reads it
    on the verify path.

## Assumptions

- A-001: git is the sharing medium. Two people signing the same act in two
  clones collide on the same response path and `authorization.toml`, so git
  reports a conflict. This Warrant adds no lock. Confidence: medium. M1's
  first step checks it on a scratch repository and records what git does.
- A-002: an assignment file under the Warrant's directory, whose digest the
  authorization request echoes, needs no change to the contract digest. The
  deliverable set works this way today. Confidence: high.
- A-003: an unassigned Warrant behaves exactly as today. Assignment is
  opt-in per Warrant, so the 136 existing Warrants are unaffected.
  Confidence: high.

## Unknowns

- **U-001 (blocking): where assignments live and who writes them.**
  - A: `docs/warrants/<alias>/assignment.toml`, echoed in the authorization
    request and bound by its signature (recommended);
  - B: standing rules in `roles.toml` ("Ada verifies everything under
    crates/"), written by a human like the rest of the register;
  - C: journal events.

  A is per-Warrant and signed. B scales but makes the register a routing
  table. C is unsigned.
- U-002 (non-blocking): whether `oh.war/assignment/v1` enters the schema
  pack. Adding it moves the pack version and every workspace digest once
  (OW-WAR-0032). Escalate before M1 lands it.
- U-003 (non-blocking): what an assignment to a person who later loses the
  role means. The draft says the act has no eligible signer and says so by
  name (`assignment.role-revoked`). The draft does not fall back to the
  others.

## Residual risks

- R-001: an assignment names a person, and a name is not a login.
  `--ssh-sign` checks the key for authorize and resolve. A verifier's
  verdict is not yet signed (OW-WAR-0138), so on the verify path the
  assignment is enforced against the name.
- R-002: a team that never assigns gets today's behavior, which includes
  row 6's residual (the register is policy, not mechanism).

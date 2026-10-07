---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-605c-73a0-af03-c3ad0e0efc55
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`war sign --ssh-sign` proves that a key listed in
`docs/authority/allowed_signers` signed the act. `authority_check` binds
that key to an actor through `roles.toml`'s `ssh_principal` (THREAT_MODEL
row 11). Three gaps remain, and OW-WAR-0096 leaves each open by name:

- **The register is performer-writable.** `roles.toml` and
  `allowed_signers` are files in the working tree. An agent that adds its
  own key and a human-kind entry makes its signature verify as a "human"
  (row 6: "policy, not mechanism"). OW-WAR-0096 built a protected store with
  signed transitions (`war authority`), but no legacy consumer reads it: its
  guide says "production cutover requires a trusted workflow to consume the
  new store and refuse legacy fallback".
- **Human presence is unverifiable.** The control is `ssh-add -c`, and
  `war` "cannot verify that `-c` was used" (row 1). A key loaded without
  it signs silently, and the record looks the same.
- **Policy that governs admissibility is performer-writable.**
  `openwarrant.toml`'s `[independence]` declaration,
  `[policy] allow_automated_resolution` (§27.3 condition 1) and
  `[verify] verifier_argv` all decide whether a Warrant may close. The
  performer can edit all three.

The product spec lists "human/session authentication, protected policy
state and credentials" as an engineering contract still to specify.

## Desired Outcome

- **A written contract.** `docs/AUTHENTICATION.md` defines what
  authenticates a human act here (a key, its binding to an actor, and
  presence) and what a "session" is. Every claim names the control or the
  operator duty that carries it.
- **The actor comes from protected state when it exists.** A repository
  that configures an OW-WAR-0096 store gets the actor-to-key binding and the
  roles from that store at its current head, checked with `authority
  allows`. `authority_check` refuses to fall back to `roles.toml` for such
  a repository (`authority.legacy-fallback`). A repository with no store
  keeps today's behavior, and `war check` says it is unprotected.
- **Presence you can verify.** A policy `require_user_presence` for
  authority acts. A FIDO (`sk-`) key's signature carries a user-presence
  flag. `war` reads it and records `presence = "verified"`. An ordinary
  key records `presence = "unverified"`, and under the policy it is refused.
- **Protected policy.** When a store is configured, the policy keys that
  govern closure are read from it. An `openwarrant.toml` value that differs
  is an error naming both values. The keys are `[independence]`,
  `allow_automated_resolution`, `require_user_presence` and the verifier
  command. The store governs.

## Non-goals

- Deploying the protected store on this machine, or creating OS accounts.
  That is the operator's, and OW-WAR-0096 names it.
- Web or remote sessions. OW-WAR-0139 consumes this contract for the LAN web
  UI. This Warrant defines "session" and ships no network login.
- Changing any signed record's meaning retroactively. Past acts keep the
  presence they had, recorded as `unverified`, and nothing is back-dated.
- Team assignment (OW-WAR-0137).
- A new signature format such as WebAuthn. The seam stays sshsig.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-605c-73a0-af03-c3ad0e0efc55
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — with a store configured, the register in the working tree grants nothing
- **scope:** `authority_check.rs` and `sign.rs` on a scratch program with
  a test-mode store and disposable keys. No claim about a deployed store's
  OS protection (R-003).
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - an agent key added to `allowed_signers` and a human-kind entry added
    to `roles.toml`, neither in the store: a signature by that key is
    refused, by name, on authorize and on resolve;
  - removing the store's grant for a human makes their next act refused at
    the new head;
  - an unreadable store gives `authority.verify-unavailable`, and no act
    is recorded;
  - `--as` naming an actor the key is not bound to is refused
    `sign.actor-key-mismatch`;
  - the positive: the store's human signs, and the record verifies;
  - without a store, `96-authority.sh` passes unchanged and `war check`
    warns `authority.unprotected`.

### OBL-002 — presence is read from the signature, never assumed
- **scope:** `presence.rs` over fixture signatures, and `sign.rs` under
  `require_user_presence`. No claim about any physical authenticator.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - an `sk` fixture signature with the presence flag records
    `verified`;
  - the same signature with the flag cleared is refused under the policy;
  - an ed25519 signature records `unverified` and is refused under the
    policy;
  - a signature with a flipped flag byte fails verification outright;
  - with the policy off, both keys sign and the record says which.

### OBL-003 — policy that governs closure cannot be changed by editing openwarrant.toml
- **scope:** `config.rs` and the v2 revision on the scratch program.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - with the store saying `allow_automated_resolution = false`, an
    `openwarrant.toml` edited to `true` gives
    `policy.unprotected-divergence`, and a policy-service resolution is
    still refused;
  - the same for `[independence]` flags set true, which
    `war check` does not count;
  - a v1 store still loads;
  - a v2 transition with an unsigned policy change is refused.

### OBL-004 — the contract says what each claim rests on
- **scope:** `docs/AUTHENTICATION.md`, `docs/THREAT_MODEL.md`.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - every "authenticated", "protected" and "presence" claim cites a plant,
    a test or an operator duty;
  - "session" is defined without any credential `war` issues;
  - rows 1 and 6 state the narrowed residual, and what remains.

## Gate Adequacy

Required at `controlled`: this Warrant decides whose signature counts. The
load-bearing obligation is OBL-001's first plant. If an agent could make its
own key verify as a human by editing two files in the working tree, every
other control in the THREAT_MODEL would rest on a convention.

**Adversarial question:** could the agent point `war` at a store it
controls? The store path is itself protected configuration. The plant sets
it through the store's own path, and the store checks the owner and mode it
can see (0096). A same-account store is refused outside test mode, and
test mode is labeled in every record it produces.

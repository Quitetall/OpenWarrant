# Human and session authentication

What makes a record in this repository a human's act, what a "session" is,
and which policy decides whether work may close. Every claim below names the
control, test, plant or operator duty that carries it. A claim with nothing
behind it is not in this document. (OW-WAR-0138.)

## Status

Built:

- the key and its binding, checked from the signature's own bytes;
- presence read from a security key's signature and recorded;
- `[policy] require_user_presence` enforced when `war sign` signs, one act or a batch.

Not built. These wait on the owner's answer to OW-WAR-0138 U-001 (how a
repository adopts protected state):

- the actor binding read from a protected store;
- `authority.legacy-fallback` and `authority.unprotected`;
- the protected policy keys read from the store and
  `policy.unprotected-divergence`;
- the v2 store revision.

Until U-001 is answered, `roles.toml`, `allowed_signers` and
`openwarrant.toml` are files in the working tree, and the performer can
write all three. THREAT_MODEL row 6 is unchanged.

## What authenticates a human act

Three things, all required. A record that says "authorized" is believed
because of all three, not because of what it says.

### 1. The key

The act's response file under `docs/authority/responses/` has a `.sig`
sidecar. `ssh-keygen -Y verify` accepts it over the file's exact bytes, under
the `oh.war/response` namespace, as the actor's principal, against
`docs/authority/allowed_signers`. Moving the signature onto another act, or
editing a byte after signing, fails.

- **Carried by:** `authority_check.rs` on the read path (every `war check`,
  §56.1 requirement 1 and `war dispatch`), and `sign.rs`, which verifies
  before anything is renamed into place.
- **Exercised by:** `conformance/plants.d/96-authority.sh`. It covers a
  missing signature, a response edited after signing, a replayed signature
  and a principal no key answers for. Also the `sign.rs` unit test
  `ssh_sign_then_verify_and_every_refusal`.

### 2. The binding: this key is this actor's

The chain runs `--as` actor → `roles.toml` `ssh_principal` → exactly one
`allowed_signers` line → one key.

- Two lines for one principal are refused (`sign.ssh-principal`). They are
  never resolved by line order.
- The actor must be `human` in the register. Otherwise the act is refused
  (`authority.actor-not-human`, `authorize.agent`, and so on).
- The key that made the signature is read from the signature blob. It must be
  the key the chain ends at. This is checked before `ssh-keygen` is asked,
  and a signature by any other key is refused `sign.actor-key-mismatch`. The
  refusal names the principals that key is bound to.
- **Carried by:** `sign.rs` (`ssh_sign_file`, `key_bound_to_principal`) for
  a single act and for `war sign --batch`.
- **Exercised by:** `58-authn.sh`, "another actor's key, refused": the
  agent's key is substituted into `ssh-keygen -Y sign`, and the act is
  refused naming `["bot"]` with nothing recorded. Also the `sign.rs` unit
  test above.
- **Residual:** the chain is only as good as the register. Today the
  register is two files the performer can edit. An agent that adds its own
  key under a human's principal passes this check. The control that remains
  is review of `docs/authority/**`, an operator duty (THREAT_MODEL row 6).
  Reading the binding from a protected store is what closes it, and that
  waits on U-001.

### 3. Presence: a person at the key

- **A security key.** A signature by an `sk-ssh-ed25519@openssh.com` or
  `sk-ecdsa-sha2-nistp256@openssh.com` key carries the authenticator's flags
  and a counter. They are inside what the authenticator signs, so they cannot
  be changed afterwards. `war` reads them from the signature blob, after
  `ssh-keygen -Y verify` has accepted the signature
  (`crates/openwarrant-core/src/presence.rs`):
  - the user-presence bit set is `presence = "verified"`;
  - anything else is `presence = "unverified"`. That covers an ordinary key,
    a security key that signed without a touch, and a blob `war` cannot read.
  - Presence is never inferred from a key type, a flag in a file or an
    absence.
- **An ordinary key.** Its signature carries nothing about presence. The
  control is the agent's confirmation dialog: the key is loaded with
  `ssh-add -c`. `war` cannot see whether `-c` was used. That is an operator
  duty (THREAT_MODEL row 1, `docs/authority/allowed_signers.example`).
- **Where presence is recorded.** A presence field in the response itself
  would be a claim nobody signed, because the signature is made over the
  response and the flags exist only once it is made. So presence is recorded
  in three places:
  - the `sign.presence` line of `war sign`'s report;
  - the attestation's predicate, as `presence` and `signing_key_type`. The
    attestation is signed, so this is a signed copy;
  - the journal's `attestation.recorded` event.
  The `.sig` stays the evidence. `war sign <target> --verify` reads presence
  from it again at any time.
- **Policy.** With `require_user_presence = true` under `[policy]` in
  `openwarrant.toml`, `war sign` refuses `sign.presence-required` for any of
  these. The refusal comes before anything is renamed into place, and the
  signature is removed.
  - an ordinary key;
  - a security key without the presence bit;
  - the terminal path, which makes no signature;
  - a `--batch` signed by any of them.
- **Carried by:** `presence.rs`, and `sign.rs` (`ssh_sign_act`) with
  `batch_cmd.rs` (OW-WAR-0138 AM-003).
- **Exercised by:** `58-authn.sh`:
  - with the policy off, both keys sign and the record says which;
  - under the policy, an ordinary key, an `sk` signature with flags `0x00`
    and a batch by an ordinary key are each refused, and nothing is recorded;
  - a flag byte flipped after signing fails verification, both at signing
    and on disk (`authority.signature-invalid`);
  - under the policy, the touched key's act is recorded and verifies.
  - Unit tests in `presence.rs` cover the fixture signatures. The `sign.rs`
    unit test `the_terminal_path_is_refused_only_under_the_presence_policy`
    covers the terminal path.
- **What was observed and what was not:**
  - The fixtures and the plant's security key are software keys that sign
    by the authenticator's rule. No physical authenticator was touched, and
    nothing here claims how any particular device sets its flags.
  - OpenSSH 10.5p1's `ssh-keygen -Y verify` accepts an `sk` signature whose
    presence bit is clear. It prints the same "Good signature" line for it,
    and it rejects `no-touch-required` and `verify-required` in
    `allowed_signers` as unknown options. The flags appear only in its `-vv`
    debug output. So `war` reads the flags itself (OW-WAR-0138 U-002,
    observed 2026-09-25).
- **Residuals:**
  - Presence is not identity. A touch proves someone touched the key, not
    who. Key custody is the operator's.
  - The policy is enforced when `war sign` signs. The read path
    (`authority_check`) does not apply it. A response signed outside
    `war sign`, with a key the agent can reach, still verifies there. Past
    acts are not re-judged by a policy set later.
  - `require_user_presence` lives in `openwarrant.toml`, which the performer
    can edit. Turning it off is a commit a reviewer sees, not something `war`
    can stop, until a store governs it (U-001).

## What a session is

A session is the human's unlocked operating-system session in which their key
is loaded:

- an ssh-agent holding the key, loaded with `-c` for an ordinary key;
- or a security key present on the machine, which asks for a touch per
  signature.

`war` issues nothing that authenticates a person. It has no login, no
session token that stands for a human, no cookie and no credential of its
own. Every human act is a signature made by that key through that agent.

- `war ui`'s per-start token is not a session credential. It guards the
  local server against other processes and forged requests (THREAT_MODEL
  row 13). A button there still ends in `war sign --ssh-sign` and the key's
  own confirmation.
- Ending a session is the operator's: lock the OS session, or remove the key
  from the agent (`ssh-add -D`). An agent socket reachable from the
  performer's shell is inside the session, which is why the dialog or touch
  is the control (THREAT_MODEL row 1).
- Web and remote sessions are not defined here. OW-WAR-0139 consumes this
  contract for the LAN web UI.

## Policy that decides whether work may close

These keys decide whether a Warrant may close, so they are the ones to
protect (OW-WAR-0138 U-003):

| Key | What it decides |
|---|---|
| `[independence]` | the §46.1 independence a repository's verification claims |
| `[policy] allow_automated_resolution` | §27.3 condition 1: whether a policy service may resolve |
| `[policy] require_user_presence` | whether a signature without presence can record an act |
| `[verify] verifier_argv` | which process gives the independent verdict |

**Today** all four are in `openwarrant.toml`, a working-tree file the
performer can edit. The control is review of that file, an operator duty.
The plan is that a protected store holds them, a differing
`openwarrant.toml` value is `policy.unprotected-divergence`, and the store's
value governs. None of that is built, and its shape waits on U-001. Under
U-001 option C there is no separate store and these keys stay unprotected.
Adding a key to this list later is a store revision.

## Claims and what carries them

| Claim | Carried by |
|---|---|
| A record is believed only with a verifying signature by the actor's key | `authority_check.rs`; `96-authority.sh` |
| The signature's key is the key bound to the actor | `sign.rs` `key_bound_to_principal`; `58-authn.sh` "another actor's key, refused" |
| The actor is human | register kind; `96-authority.sh` "an agent named as the authorizer" |
| Presence is read from the signature, never assumed | `presence.rs` unit tests; `58-authn.sh` |
| Under the policy, a signature without presence records nothing | `sign.rs` `ssh_sign_act`, `batch_cmd.rs`; `58-authn.sh` |
| An ordinary key's dialog was shown | operator duty (`ssh-add -c`), THREAT_MODEL row 1 |
| The register and policy are outside the performer's reach | not established. Operator review today; a store after U-001 |
| A session is the human's unlocked OS session with a loaded key | definition. `war` issues no credential (THREAT_MODEL row 13) |

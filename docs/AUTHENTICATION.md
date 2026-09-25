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

Built CONTINGENTLY, on a branch not for merge until the owner answers
OW-WAR-0138 U-001 (how a repository adopts protected state), against the
basis's recommended option A — opt-in per repository:

- the actor binding, kind and role read from a protected store's current
  head, with `authority.legacy-fallback` for a record the store does not back;
- `authority.unprotected`, warned once by `war check` in a repository with no
  store;
- the protected policy keys read from the store, and
  `policy.unprotected-divergence` for a file value that differs;
- the v2 store revision (actor kind per principal, a policy table).

A repository with no `[authority]` table reads exactly as before, plus the
warning: `roles.toml`, `allowed_signers` and `openwarrant.toml` are files in
the working tree, and the performer can write all three. This repository has
no store. Adopting one is the owner's act (`docs/cli/authority.md`,
"Cutover").

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
- **With a store** (`[authority] store`, contingent on U-001 option A): the
  chain runs `--as` actor → the store principal whose `actor` is that name →
  its key, at the store's current head, and the principal must be `human`
  there and hold the act's role (`authorizer` for authorize, correct and
  accept; `resolver` for resolve). `roles.toml` and `allowed_signers` grant
  nothing:
  - a record that verifies only against them is `authority.legacy-fallback`,
    on the read path and at signing, with nothing recorded;
  - a grant the store withdrew is `authority.not-granted`;
  - a store that cannot be read, that the reading process can write, or that
    is a test store named as protected is `authority.verify-unavailable` —
    never a fall back;
  - a grant read at one head and a store at another by the time the key has
    signed is `authority.stale-head`, and the signature is removed. This one
    is in the code (`sign.rs` `ssh_sign_act`) and not exercised: no plant
    moves the store while the key is being asked.
  - **Carried by:** `authority_check.rs` (`verify_store`, `signer_for`) and
    `authority_cmd/store.rs` (`read_current`, whose guard refuses a store the
    execution account owns or any group or other can write).
  - **Exercised by:** `58-authn.sh`, the store half: the performer's own key
    registered as a human in the two working-tree files verifies without a
    store and is `authority.legacy-fallback` with one, for an authorization
    and a forged resolution, and a new act by it is refused; the store's
    human signs and verifies; a revoked grant; a writable, an unreadable and
    a mis-moded store. The `store.rs` unit test covers the owner and mode
    rule for a separate account, which no plant can create.
- **Residual:** without a store the chain is only as good as the register,
  two files the performer can edit. An agent that adds its own key under a
  human's principal passes this check; review of `docs/authority/**` is the
  operator's control (THREAT_MODEL row 6), and `war check` says so
  (`authority.unprotected`). With a store:
  - the store is as protected as the account holding it; `war` checks the
    owner and mode it can see and cannot prove the deployment (R-003);
  - the `[authority]` table is in `openwarrant.toml`; removing it is a
    reviewed commit that brings the warning back, and an older `war` that
    does not know the table ignores it unless `[project] requires_war`
    refuses that `war`;
  - `roles.toml` still says who is eligible to be asked; with a store it is
    necessary and never sufficient;
  - attestations are verified against the working-tree `allowed_signers`
    (`attest.rs`), not the store;
  - the read path judges every record at the store's current head, so
    withdrawing a grant or rotating a key also re-judges acts signed before
    it (`58-authn.sh` "revocation reaches the read path"). Whether history
    should be judged at the head it was signed under is the owner's to
    decide; no timestamp here is authenticated, so it is not guessed.

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
  - Without a store, `require_user_presence` lives in `openwarrant.toml`,
    which the performer can edit: turning it off is a commit a reviewer
    sees. With a store the store's value governs; `58-authn.sh` "the store
    requires presence" observes an ordinary key refused while the file says
    `false`.

## What a session is

A session is the human's unlocked operating-system session in which their key
is loaded:

- an ssh-agent holding the key, loaded with `-c` for an ordinary key;
- or a security key present on the machine, which asks for a touch per
  signature.

`war` issues nothing that authenticates a person. It has no login and no
session token that stands for a human. Every human act is a signature made by
that key through that agent.

`war ui` issues two things that authenticate a request, and neither
authenticates a person or can sign.

- **The loopback token.** A 256-bit token per `war ui` start, printed once in
  the link's fragment and carried by the page in memory as
  `Authorization: Bearer`. Never a cookie, storage or a URL. It guards the
  127.0.0.1 server against other local processes and forged requests. A
  button there still ends in `war sign --ssh-sign` and the key's own
  confirmation.
  - **Carried by:** `webui/mod.rs`, the loopback route (every `/api/`
    request compared with `same`).
  - **Exercised by:** `63-webui.sh` (401 without or with a wrong token) and
    the `webui` unit test `tokens_are_long_random_and_compared_in_constant_time`.
  - THREAT_MODEL row 13a.
- **The device credential** (`war ui --lan`, OW-WAR-0139). It authenticates
  a paired device, not a person, to **read** the program and **request**:
  run an `auto` remedy, or mark a signing act "requested from `<device>`" in
  the host's queue. It never signs and never causes a signature. A device's
  POST naming a signing act is refused `act.host-only` before anything runs.
  "Ask at the host" starts nothing; the signature is still
  `war sign … --ssh-sign` typed at the host, under that key's dialog or
  touch.
  - **Issued:** only after a `y` typed at the host's terminal answers a
    pairing prompt naming the device's address and user agent. The prompt
    is reached with a single-use code that expires in five minutes.
  - **Form:** 256 bits from the OS, sent once as the `__Host-war_device`
    cookie (`Secure; HttpOnly; SameSite=Strict; Path=/`), over TLS only.
  - **Storage:** the host keeps only its SHA-256, with the device's address,
    user agent and a 30-day expiry, in
    `$XDG_STATE_HOME/openwarrant/ui-devices.json` (mode 0600). The file is
    per user and never under the repository. It is not a record and grants
    nothing under `docs/authority/`.
  - **Checked:** the file is read on every request, so
    `war ui devices --revoke <id>` takes effect at once. Deleting the file
    revokes every device. On the LAN no address is trusted, loopback
    included, and the loopback token means nothing there.
  - **Carried by:**
    - `webui/pairing.rs`: `COOKIE`, `Store::issue`, `Store::authenticate`
      (revoked, expired, unknown), `Store::open` (refuses a file under the
      repository, `ui.devices-in-repository`), the read's mode check
      (`ui.devices-mode`), `Codes::take` and `Nonces::spend`;
    - `webui/mod.rs`: `serve_lan` (no `/api/` without a device credential),
      `start_act` (a verb of `sign` from a device is `act.host-only`),
      `device_queue` (a device's rows carry no act id) and
      `request_at_host` (marks, starts nothing);
    - `webui/tls.rs`: `Tls::operator` / `Tls::self_signed`, and `accept`,
      where a failed handshake gets no HTTP.
  - **Exercised by:** `59-webui-lan.sh`:
    - OBL-002: a missing, forged, revoked or expired credential, and the
      loopback token, each 401; the device file 0600, outside the
      repository, hashes only; a reused or expired pairing code; a replayed
      nonce 409;
    - OBL-003: the device's queue without act ids; a signing id 403
      `act.host-only` with no `war sign` child and nothing signed; a request
      marked at the host with nothing run;
    - OBL-004: `n`, no answer and no terminal each issue nothing.
    - Unit tests: `pairing.rs`
      (`a_credential_is_stored_only_as_its_hash_in_a_private_file`,
      `a_device_file_under_the_repository_is_refused`,
      `a_pairing_code_works_once_and_not_after_expiry`,
      `a_nonce_is_spent_once_and_only_by_its_device`) and `tls.rs`
      (`a_self_signed_certificate_serves_its_name_and_no_other`).
  - **Residuals** (THREAT_MODEL row 13b):
    - a device stolen while unlocked reads and runs `auto` remedies until
      it is revoked;
    - the pairing prompt is a TTY guard that a pty can answer (row 2), so
      what it buys is a credential that cannot sign;
    - the cookie is scoped to the host name, not the port.
    - That a device cannot sign rests on `start_act` refusing the `sign`
      verb and on `auto` remedies never being a signing verb (OW-WAR-0112).
      Nothing in `webui/` names `ssh-keygen` or the agent socket.

Ending a session:

- Ending a session is the operator's: lock the OS session, or remove the key
  from the agent (`ssh-add -D`). An agent socket reachable from the
  performer's shell is inside the session, which is why the dialog or touch
  is the control (THREAT_MODEL row 1).
- Ending a device's reach is `war ui devices --revoke <id>`, or deleting the
  device file. Restarting `war ui` rotates the loopback token.

## Policy that decides whether work may close

These keys decide whether a Warrant may close, so they are the ones to
protect (OW-WAR-0138 U-003):

| Key | What it decides |
|---|---|
| `[independence]` | the §46.1 independence a repository's verification claims |
| `[policy] allow_automated_resolution` | §27.3 condition 1: whether a policy service may resolve |
| `[policy] require_user_presence` | whether a signature without presence can record an act |
| `[verify] verifier_argv` | which process gives the independent verdict |

**Without a store** all four are in `openwarrant.toml`, a working-tree file
the performer can edit, and `war check` warns `authority.unprotected`. The
control is review of that file, an operator duty.

**With a store** (contingent on U-001 option A), the v2 revision's `policy`
table holds them. When the repository is opened the store's values replace
the file's for every consumer (`repo.rs`, `config.rs`
`govern_from_store`), and each file value that differs is
`policy.unprotected-divergence`, an error naming both values. A configured
store that gives no policy — unreadable, or still at a v1 head — closes every
key to its most restrictive value (no automated resolution, presence
required, no verifier, independence undeclared), never the file's.

- **Carried by:** `config.rs` (`govern_from_store`, `govern_fail_closed`),
  `authority_check.rs` (`govern`, `protection_report`), and
  `authority_transition.rs` (the v2 revision; a policy change moves the
  proposal's signing bytes, so it needs the admin's signature like any
  other).
- **Exercised by:** `58-authn.sh`: `allow_automated_resolution` and
  `[independence]` edited against the store are divergences, and `war check`
  reports the store's undeclared independence, not the file's flags; a v1
  store loads and authorizes nobody (`authority.store-v1`); a v1-to-v2
  change with no signature, and with a signature moved onto a policy the
  admin did not sign, are refused and the head does not move. The `config.rs`
  unit test shows a policy-service resolution refused under the store's
  `false` while the file says `true`; no plant reaches a policy-service
  resolution end to end, because that needs a Warrant meeting all thirteen
  §56.1 requirements.

Under U-001 option C there is no separate store and these keys stay
unprotected. Adding a key to this list later is a store revision.

## Claims and what carries them

| Claim | Carried by |
|---|---|
| A record is believed only with a verifying signature by the actor's key | `authority_check.rs`; `96-authority.sh` |
| The signature's key is the key bound to the actor | `sign.rs` `key_bound_to_principal`; `58-authn.sh` "another actor's key, refused" |
| The actor is human | register kind; `96-authority.sh` "an agent named as the authorizer" |
| Presence is read from the signature, never assumed | `presence.rs` unit tests; `58-authn.sh` |
| Under the policy, a signature without presence records nothing | `sign.rs` `ssh_sign_act`, `batch_cmd.rs`; `58-authn.sh` |
| An ordinary key's dialog was shown | operator duty (`ssh-add -c`), THREAT_MODEL row 1 |
| The register and policy are outside the performer's reach | without a store: not established, operator review (`authority.unprotected`). With a store (contingent, U-001 A): `authority_check.rs`, `config.rs`, `58-authn.sh` store half; the store's own protection is the operator's (R-003) |
| A record the store does not back is refused, never fallen back from | `authority.legacy-fallback`, `authority.verify-unavailable`; `58-authn.sh` |
| A session is the human's unlocked OS session with a loaded key | definition. `war` issues no credential that authenticates a person or can sign |
| The loopback page's token guards the local server and is never a cookie | `webui/mod.rs` loopback route; `63-webui.sh`; THREAT_MODEL row 13a |
| A paired device's credential reads and requests, and never signs or causes a signature | `pairing.rs` `Store::authenticate`, `mod.rs` `serve_lan`, `start_act` (`act.host-only`), `request_at_host`; `59-webui-lan.sh` OBL-002, OBL-003; THREAT_MODEL row 13b |
| The device credential is issued only by a `y` typed at the host, and stored only as a hash outside the repository | `mod.rs` `pair`, `pairing.rs` `Store::issue`, `Store::open`; `59-webui-lan.sh` OBL-002, OBL-004; `pairing.rs` unit tests |

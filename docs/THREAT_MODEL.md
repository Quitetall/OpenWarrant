# Threat model

What OpenWarrant defends, what it does not, and which control carries each
claim. Twelve entries; each names the threat, the control, the residual, and what
exercises the control today. "Exercised by" is a test, a plant, or an operator
duty — never a sentence in a document. Where the honest answer is "the
operator", it says so.

The asset is a **record that says a human authorized, resolved, corrected or
accepted something**. The adversary that matters most is not hostile: it is an
AI agent completing the loop by helpfulness — authorizing its own work,
verifying its own delivery, editing a pinned file, or signing as the human
because the socket was there. Hostile actors are covered where the same
control holds them; where it does not, the entry says so.

| # | Threat | Control | Residual | Exercised by |
|---|---|---|---|---|
| 1 | **The agent reaches `SSH_AUTH_SOCK`** and signs as the human. Inside an agent session the shell and the human share an ssh agent. | The key is loaded with `ssh-add -c`: every signature asks the human through the agent's confirmation dialog, which no shell can answer. `--ssh-sign` never touches the private key (`ssh-keygen -Y sign` asks the agent). Every signature is a committed artifact (`docs/authority/responses/*.sig`), so a signature made without a dialog is at least visible in history. A FIDO key (`sk-` type) makes presence verifiable (OW-WAR-0138): its signature carries the authenticator's user-presence flag, covered by the signature. `war` reads it from the signature after `ssh-keygen -Y verify` accepts it, which OpenSSH does not do itself. The flag is recorded as `presence = "verified"` or `"unverified"` in the `sign.presence` report line, the attestation predicate and the journal. Under `[policy] require_user_presence`, an ordinary key, an `sk` signature without the flag, the terminal path and a batch by any of them are refused `sign.presence-required` before anything is recorded. | Narrowed by OW-WAR-0138: with a security key and the policy set, a signature made with no one at the key is refused, and without the policy it is recorded as `unverified`. What remains: `war` cannot verify that `-c` was used for an ordinary key, and such a key signs silently when the policy is off. Presence is not identity (a touch proves someone touched the key). The policy is enforced when `war sign` signs, not on the read path. Without a store it lives in `openwarrant.toml`, which the performer can edit; with `[authority] store` (contingent on U-001 option A) the store's value governs and a differing file value is `policy.unprotected-divergence`, and `58-authn.sh` observes an ordinary key refused under a store's policy while the file says false. Attestations (OW-ADR-0015) make each signature a self-describing artifact but do not close this. | Operator duty for `-c`. `sign.rs` module doc states it; `--ssh-sign` end to end was measured Allow→"Good signature", Deny→refused on 2026-09-11. Presence: `conformance/plants.d/58-authn.sh` uses a software key shaped as a security key, so no authenticator is touched. With the policy off, both keys sign and the record says which. Under the policy, the ordinary key, flags `0x00` and a batch are each refused with nothing recorded, and a flipped flag byte fails verification. `presence.rs` unit tests cover the fixture signatures; `docs/AUTHENTICATION.md` has the contract. |
| 2 | **A pseudo-terminal defeats the TTY check.** `war sign` at a terminal requires `isatty`; an agent can allocate a pty. | Accepted. The TTY path is a guard against the loop being completed by accident, not a cryptographic one; trust rests on the key and the confirmation dialog (entry 1). `signed_via = "tty"` is recorded so a reader can tell the two forms apart. | An agent that deliberately allocates a pty can pass the TTY guard. The remedy is `--ssh-sign` with `ssh-add -c`; the TTY path exists for a human at a real terminal. The same guard fronts `war init`'s writing of `roles.toml` and `allowed_signers` (OW-WAR-0112): a pty defeats it there too, and the files it would write grant an agent `performer` only, so what a pty buys is a register naming a human the agent typed — which the key, not the register, must then back. | Unit test: `at_a_terminal()` is false under `cargo test`; the no-terminal refusal is tested. |
| 3 | **The askpass helper accepts on Deny.** A helper that exits 0 for any button makes the dialog decorative. | The operator's askpass helper parses the agent's `CONFIRM` prompt kind and returns non-zero on Deny; a helper that answers every prompt with 0 is the bug entry 1's dialog exists to prevent. | The helper lives in the operator's environment, not in this repository; `war` sees only "signature verified" or "refused". | Operator's own test of Deny→refused (measured 2026-09-11); the pinentry trap is documented in `docs/authority/allowed_signers.example`. Not exercisable by this repository's gate. |
| 4 | **A newline in the confirmation prompt** lets the prompt carry a second, forged line ("Allow: also sign X"). | The helper receives the prompt with newlines encoded (`%0A`) and renders one line; the thing being signed is shown by `war sign` on the terminal before the dialog, from the record's own facts, never from free text. | Rendering is the helper's. | Operator's helper (the `%0A` rule is in `docs/authority/allowed_signers.example`); `war sign --show` renders the same facts deterministically (unit-tested). |
| 5 | **The agent's environment lacks `SSH_ASKPASS` / `DISPLAY`**, so the dialog never appears and `ssh-keygen` either hangs or fails opaquely. | `war sign --ssh-sign` reports what it observed: the `ssh-keygen` exit status and stderr verbatim in the refusal. It never falls back to a prompt-less path. | A hung dialog is a hung command; the operator kills it. | Refusal path unit-tested; the `sign.ssh-*` rules name the cause; the systemd drop-in need is in `docs/authority/allowed_signers.example`. |
| 6 | **The authority register is edited by the agent** (`docs/authority/roles.toml`, `allowed_signers`): grant itself a role, add its own key. | Without a store: both files are human-written by policy and committed: a change is a commit, visible in review, and `war sign` verifies against the committed `allowed_signers`. An agent-kind actor is refused by kind on every act (§27.2, `authorize.agent`, `correction.agent`, …) whatever the register says about it. Duplicate principals are refused (`sign.ssh-principal`), and `war check` warns `authority.unprotected` once. With `[authority] store` in `openwarrant.toml` (OW-WAR-0138; CONTINGENT on the owner answering U-001 option A, not decided): the actor's principal, key, kind and the role each act needs come from the store's current v2 head, on the read path (`authority_check`) and when `war sign` signs, one act or a batch. The two working-tree files grant nothing: a record that verifies only against them is `authority.legacy-fallback`, a signature by a key the store does not bind is `sign.actor-key-mismatch`, a withdrawn grant is `authority.not-granted`, and a store that cannot be read, is writable by the reader, or is a test store presented as protected is `authority.verify-unavailable` with nothing recorded — never a fall back. The protected policy keys are read from the same head; a differing `openwarrant.toml` value is `policy.unprotected-divergence`. | Without a store: policy, not mechanism — nothing stops a commit that edits the register, and a branch-protection rule requiring a human review of `docs/authority/**` is the operator's control. With a store, narrowed to the store's own protection: `war` checks the owner and mode it can see and cannot prove a separate account (R-003, `isolation_enforced: false`); the `[authority]` table is itself in `openwarrant.toml`, so removing it is a reviewed commit that turns the warning back on (and an older `war` ignores the table unless `[project] requires_war` refuses it); `roles.toml` still lists who is eligible, so a store human must also appear there (necessary, never sufficient); attestations are still verified against the working-tree `allowed_signers`; and the read path judges every record at the current head, so a withdrawn grant also re-judges acts signed before it (a decision left to the owner). | Plants: agent actor refused on authorize/resolve/correct/accept; duplicate principal refused. Store (`58-authn.sh`, a same-account test store sealed read-only; no second account is created or observed): unconfigured warns `authority.unprotected` and the register's forged human verifies; configured, the same forged authorization and resolution are `authority.legacy-fallback` and a new act by that human is refused; the store's human signs and verifies; a revoked grant is `authority.not-granted`; a writable, unreadable, or mis-moded store is `authority.verify-unavailable` with nothing recorded; unsetting the store restores the legacy path, warned. `store.rs` unit test for the owner/mode rule. |
| 7 | **Write-before-confirm.** A response file written to `docs/authority/responses/` before the signature verifies would be a record of an act that never happened. | Verification against `allowed_signers` happens before anything is renamed into place; a signature that does not verify writes nothing. The draft lives beside the final name until then. | None known. | Unit tests in `sign.rs` (verify-before-rename); plant `sign.ssh-verify` refused. |
| 8 | **Same-digest overwrite / archival.** A second response for the same revision could silently replace the first, or an unsent draft could be archived as if sent. | A response for an earlier revision is retired to `<name>.<tag>.response.toml` beside its `.sig`, never deleted; a second retirement onto an existing archive name is refused, not clobbered; "same digest" is distinguished from "no digest found" so nothing unsent is archived. | None known. | Unit tests (`sign.rs`: refused-never-archived, second retirement refused). |
| 9 | **The correction act as a laundering path.** A resolved Warrant's pinned file is edited and a "correction" recorded to make the drift legal. | A correction is a human act (`correction.agent` refuses the agent by kind); it is refused when nothing drifted (`correction.no-drift`), when the new digest is not the file's bytes (`correction.stale`), when the superseded digest is not the chain head (`correction.superseded-mismatch`), when the reason is empty, when the Warrant is not resolved; the journal event carries the record's digest so a later edit of the correction file is `correction.edited`. `deliverables.toml` is never regenerated — the resolution's digest still binds it. | A human can sign a correction whose reason is false. That is a lie in a signed record, which is what the record is for. | Plants `plants.d/64-corrections.sh`: six refusals and the positive `deliverable.corrected`; OBL-005 (superseded digest removed → `correction.malformed`). |
| 10 | **Attestation replay across namespaces.** A signature made for one purpose (a response) presented as another (an attestation), or an attestation for one record presented for another. | Signatures are namespaced: responses sign under `oh.war/response`; attestations sign under `oh.war/dsse` over the DSSE PAE, which binds payload type and length, and the in-toto subjects carry the record digests. A signature under one namespace does not verify under the other (OW-ADR-0015). | An `allowed_signers` line without the `oh.war/dsse` namespace refuses the attestation, not the act; the act stands unattested with `attest.not-emitted`. | Plant `plants.d/67-attest.sh`: a response-namespace signature replayed as DSSE is refused; a moved payload byte, an edited subject and an unknown principal are each refused by name. Unit test in `attest.rs` against a generated key. |

| 11 | **The record believed for its contents.** An agent writes `authorization.toml` saying `authorized`, naming the owner as authorizer, and nothing ever asks who signed it. Demonstrated 2026-09-19 against the published 1.0.0-alpha.1: a hand-written record whose `meaning` read "FORGED BY AN AGENT. No human saw this" produced `war check` with zero errors and `PASS §56.1 exact authorized Contract Revision`. | Every authority record is now read through `authority_check`: a response for THAT act, over THAT digest, whose `.sig` `ssh-keygen -Y verify` accepts as the principal `roles.toml` binds to the actor, who is `human` there. Authorizations, resolutions, corrections and SAS acceptances all pass through it; §56.1 requirement 1 and `war dispatch` depend on the verdict, so an unsigned record stops the work it claims to authorize. Fail-closed by construction: a missing `ssh-keygen` is `authority.verify-unavailable`, never a pass. | An agent can still WRITE the record — the refusal is on the read path, which is the point: the file is a draft until a human signs it. Response filenames are per act, so one signature cannot answer for two. Pre-enforcement acts are signable as themselves (`*.signature-supplied`) and nothing is back-dated. | Plants `plants.d/96-authority.sh`: a missing signature, a response edited after signing, a signature replayed onto another Warrant, a principal no key answers for, an agent named as authorizer, an unsigned resolution, a Dispatch of an unsigned contract, `--prototype` naming the missing authority in the packet, an authorization with no response at all, and requirement 1 reported unmet — plus the positive that the committed corpus verifies. |
| 12 | **Ownership claimed after signing.** Under OW-ADR-0021 the Warrant that governs a path is the latest AUTHORIZED one that declares it, and `deliverables.toml` is not inside the contract digest — so an agent could add a path to the manifest after the human signed, or hand-write a response naming a set the human never saw, and then edit a file every earlier Warrant had pinned. | The request lists every `(id, path)` and their set digest; the response the human signs echoes the digest; ingest refuses a moved set (`authorize.stale-deliverables`) before any signature is verified; the set is copied into `authorization.toml`, which is already a DSSE subject, so an edit to `owned` afterwards is `attest.subject-drift`; a record with no set owns nothing (every record made before the ADR); a path in the manifest and not in the recorded set is `deliverable.undeclared-at-authorization` and is not governed; an authorization dated before an existing owner of a path is refused (`authorize.time-before-owner`). The signing screen prints every path under "grants ownership of", so what is granted is read, not inferred from a digest. | A signer who does not read the list they sign — the same residual as row 11, in the same words. A hand-written response carrying no set digest records the set unsigned and says so (`authorize.deliverables-unsigned`); the paths are governed, the signature does not cover them, and `war pins` reports it. Ordering is by a locally stamped effective time (§67.2); the refusal covers the case seen, not a clock set back between two signings. | `conformance/plants.d/98-ownership.sh` (OW-WAR-0112 M1): the moved-set response refused with nothing written; the post-signing path unowned; the edited `owned` list as subject drift; the out-of-order time refused; the historical pin passing as `deliverable.superseded-by` while an undeclared edit still drifts. Unit tests in `ownership.rs` for the set digest and the ordering. |
| 13a | **A click in the web UI signs — loopback.** `war ui` (OW-WAR-0116) starts `war sign <target> --ssh-sign` from a button; a local process that obtains the session token, or a key loaded without `ssh-add -c`, turns a click — or a forged request — into a signature. | The server binds 127.0.0.1 only; checks Host (DNS rebinding) and Origin; requires a per-start 256-bit token held in the page's memory only (never cookie, storage or URL); accepts acts by POST with a matching Origin; runs only an act from an allowlist it built from the queue's dry-run verdicts, never an argv from the browser; dry-runs that exact act again before running it; runs one act at a time. The key's `ssh-add -c` dialog remains the human act. | A local process that reads the token from the owner's terminal scrollback can start an allowlisted act; with `-c` the owner still sees the dialog and can refuse. Without `-c`, entry 1's residual applies to every click. `war ui --lan` leaves this page as it was and runs it beside the LAN listener (13b); `59-webui-lan.sh` checks that its headers are byte-identical to the pre-LAN build. | `conformance/plants.d/63-webui.sh`: 401 without or with a wrong token, 400 on a foreign Host, 403 on a foreign or missing Origin, 405 on a non-POST act, 403 on an id off the allowlist with nothing run, 400 on a body carrying an argv, 431 on oversized headers, a CSP without `unsafe-inline`, a loopback-only socket, the exact act run for a valid id with no agent socket and nothing signed; `webui` unit tests for the CSP, the page's lack of inline code, the token and the act body. |
| 13b | **A device on the LAN acts, or a click there signs.** `war ui --lan` (OW-WAR-0139, built against U-001 A and U-002 A): another host on the network reads the queue or starts an act; a stolen or replayed pairing code or credential does; a remote click raises the host's key dialog, which whoever sits at the host could answer for someone else. | Opt-in and TLS only: `--lan` refuses to start without `--cert`/`--key` or the labelled `--self-signed` fallback (`ui.lan-needs-tls`, nothing bound), binds only the typed address, and a failed handshake — plain HTTP included — gets no HTTP. Host exactly `<name>:<port>` (else 421), Origin exactly `https://<name>:<port>`, HSTS on every response. No request is trusted for its address, loopback included, and the loopback token means nothing there: every `/api/` request needs a device credential. Pairing: a single-use code that expires in five minutes, confirmed by a `y` typed at the host's terminal, which names the device's address and user agent; no answer, `n` or no terminal issues nothing. The credential is 256 bits in a `__Host-`, `Secure`, `HttpOnly`, `SameSite=Strict` cookie; the host keeps its SHA-256 only, per user, mode 0600, outside the repository, read on every request so `war ui devices --revoke` is immediate; it expires in 30 days. Every act carries a single-use nonce issued to that device. Under U-001 A a device's queue carries no act id, a POST naming a signing id is 403 `act.host-only` before anything runs, and "Ask at the host" only marks the act requested there; a device can start an `auto` remedy and nothing else. | A device stolen while unlocked reads the program and runs automatic remedies until revoked (R-001); it cannot sign. Other LAN hosts can still connect and be refused; the server bounds each request as on loopback and serves at most 16 LAN connections at once, but nothing limits their rate (R-002). The pairing prompt is a TTY guard, and a pty answers it (entry 2, R-003): what that buys is a credential that cannot sign. With `--self-signed` the first visit shows a browser warning and only a human comparing fingerprints stands between the device and an impostor on the network (A-002). The cookie is scoped to the host name, not the port: another HTTPS service on the same name receives it. | `conformance/plants.d/59-webui-lan.sh`: no certificate or no key refused with nothing bound; plain HTTP answered with no HTTP; 421 on a wrong or loopback Host, 403 on a foreign or `http://` Origin; HSTS and the CSP on 200, 401, 404 and 421; the socket on the given address only; 401 with no credential, the loopback token, a forged, a revoked and an expired credential; a reused and an expired pairing code refused; `n`, no answer and no terminal each issue nothing, and the prompt names the address; the device file 0600, outside the repository, hashes only; the device's queue without act ids; a signing id 403 `act.host-only` with no act, no `war sign` child and nothing signed; a request marked at the host with nothing run; an `auto` remedy run once and its replayed nonce 409; the self-signed fallback labelled and its fingerprint the one served. `pairing.rs` and `tls.rs` unit tests: the hash-only store, revocation and expiry, single-use codes and nonces, and the fallback certificate completing a real handshake for its name only. |
| 14 | **Who reviews is chosen after the human signed, or a verdict is written under another person's name.** An agent edits a Warrant's `assignment.toml` after authorization to name itself verifier, or deletes it so every eligible signer is back in play; or writes a verification response naming the assigned human. (OW-WAR-0137) | The assignment is listed in the authorization request with its digest, echoed by the signed response, and recorded in the authorization's journal event; a response echoing another digest is refused `authorize.stale-assignment`, one echoing none `authorize.assignment-unsigned`, before anything is written. An assignment only intersects with `roles.toml`: an actor without the role is refused `assignment.role-missing`, an agent resolver `assignment.agent-prohibited`, the performer as verifier `assignment.self-verify`. After authorization an edited, added or removed file is `assignment.moved`, reported by `war check`, and `war sign`, `war resolve --response` and `war verify` then let nobody act — a removal never widens. On an assigned Warrant a verdict from anyone else is refused `verify.not-assigned`, and one from an assignee without `verifier` `verify.role-missing`, with nothing written. | A verdict is not yet signed: on the verify path the assignment is enforced against the NAME in the response, so a forged verdict naming the assigned verifier is recorded (OW-WAR-0138 signs human verdicts). `authorization.toml`'s schema is in the published pack, so the signed digest is read from the authorization response under `docs/authority/responses/` — the bytes under the human's key, so stripping the journal's copy as well as the file still reads as a removal — and from the journal only when no response is on disk. | `conformance/plants.d/57-teams.sh`: the request lists the assignment and digest; `authorize.stale-assignment` and `authorize.assignment-unsigned` with no `authorization.toml` written; `assignment.role-missing`, `assignment.agent-prohibited`, `assignment.self-verify`; `assignment.moved` for an edit and for a removal (also with the journal's digest stripped), on both `war sign` and `war verify`; `war check` naming `assignment.moved` for the edit and the removal; `resolution.not-assigned` for a hand-written response with nothing written; `verify.not-assigned` and `verify.role-missing` with no file under `verifications/`; the assigned verdict recorded and an unassigned Warrant unchanged. Unit tests in `assignment.rs` pin that narrowing never adds an actor. |

## What is out of scope, stated

- **A hostile human with the signing key.** Every control above assumes the
  person holding the key is the person the register names. Key custody is the
  operator's.
- **The git history itself.** A force-push that removes a signed response
  removes the record. Branch protection and a second remote are the
  operator's controls; attestations make removal detectable by whoever kept a
  copy, not preventable.
- **The MCP client.** `war mcp` registers no signing or ingesting tool and
  states its non-authority at `initialize`, so a compromised client can do
  what an agent can do at the CLI and nothing more. It cannot reach entries
  1–5 at all: there is no path from a tool call to `ssh-keygen`.
- **Timing and availability.** Nothing here is a control against denial of
  service; a stuck dialog is a stuck command.

## How this document stays true

Each "Exercised by" cell names a test or plant that exists, or an operator duty
that cannot be one. When a control changes, the entry changes in the same
commit; when a residual closes, the entry says which slice closed it. A row
whose control is only a sentence is a row to be suspicious of.

## Candidate signed authority transitions (OW-WAR-0096)

The `war authority` surface is separate from the legacy `roles.toml` and
`allowed_signers` loader described above. Installing this candidate does not
silently migrate trust or make existing signatures authenticate historical role
grants. `war sign` and the read path (`authority_check`) consume a store only
when a repository configures one with `[authority] store` (OW-WAR-0138,
contingent on the owner answering U-001 option A), and then refuse legacy
fallback (row 6; `docs/cli/authority.md`, "Cutover"). Every other command that
reads `roles.toml` — eligibility lists, verification assignment — still reads
it; with a store it is necessary and grants nothing on its own.

| Threat | Control | Evidence and residual |
|---|---|---|
| An agent edits roles or substitutes its own key | Proposals are inert; activation checks signatures against the previous trusted revision and administration/recovery role | SDK and CLI controls refuse unsigned, self-granted and wrong-key updates. Human custody of administrative/recovery keys is still an operator responsibility; `human_review_established` is always false here. |
| A signature approves different work or repository | Dedicated signing namespace and domain-separated canonical proposal bytes include repository, parent digest, operation and full next revision | Tampered-proposal CLI refusal and SDK wrong-repository/parent controls. Unknown formats and noncanonical encodings refuse. |
| An attacker substitutes the signature verifier through PATH | Verification invokes `/usr/bin/ssh-keygen` with a cleared environment | CLI poisoned-PATH control refuses a fake signature despite a replacement executable returning success. System executable, dynamic loader, CLI and host OS must be protected from the execution account. |
| Old signed state is replayed or two writers race | Current accepted state is outside the execution account; activation locks, reloads and checks parent, then atomically replaces one snapshot containing history and head | Replay and concurrent activation controls. A file-store operator able to replace trusted state can roll it back; signature chains alone do not prevent this. |
| Crash splits history from current head | One synced snapshot is renamed, followed by directory sync | Old/new complete file is the recovery unit. Orphan pending files are never accepted. A failure after rename but before successful directory sync has uncertain durability: caller reads status before retrying. Physical disk-loss/power-cut qualification is not established by unit tests. |
| “Outside the repo” is mistaken for isolation | Normal store mode requires a distinct configured execution UID, private owner directory and non-writable ancestors; test mode is explicit | Same-account bootstrap and mode mismatch refuse. `isolation_enforced` stays false: these checks cannot prove actual account deployment, ACLs, sudo rules, host privileges or protected executable installation. |
| Agent reaches store, signing socket or host credentials | Linux reference runner exposes only runtime libraries and dedicated task workspace; separate namespaces, empty environment, no network/home/socket mounts | Local sandbox probe observed task writes succeed and outside authority writes fail. Workspace must exclude authority and credentials. Runner/bwrap installation must be protected. Unrestricted agents outside this runner are not contained. |
| Receipt names a different transition, signer set or sequence | Receipt subject fields are checked against the retained, signature-verified transition; receipt and head share one atomic write | CLI plants wrong digest, previous/new head, signer set, zero/out-of-range sequence. Operator storage protects observations: UID/time are unsigned metadata, exports do not authenticate their origin or clock, and old snapshots may lack receipts. Missing receipt counts stay visible; no invented backfill. |
| Migration rewrites or blesses history | Bootstrap retains exact legacy role/key bytes; independently supplied revision digest establishes the new baseline | CLI migration/history export checks preserve originals. Bootstrap must be performed by the operator through a protected path; it is never inferred from the legacy key list. |

`authority allows` checks a current role and exact head. It does not authenticate
its caller or perform a privileged action. A trusted workflow must authenticate
the acting principal and enforce the result atomically with, or recheck it before,
the protected action. Removing keys affects future transitions; prior signed
history is verified against the key set that was current for that transition.

## Reference workflow dispute decisions (OW-WAR-0107)

The reference web application's dispute credentials select a configured human
responder with governing scope for the exact Warrant. They do not prove human
presence or grant the OpenWarrant assurance mark. Operators must protect responder
credentials, configuration and control storage from the performer. Same-account
file access is not an isolation boundary.

A retained repair decision binds the question, observation, candidate and responder
configuration. Dispatch rechecks current source, policy, candidate, writer state
and aggregate budgets. Removing responder authority makes the old decision
inapplicable. A human repair decision cannot override unknown observations,
missing evidence or repair limits, and cannot rewrite the original FAIL.
`test_verifier_repair`, `test_verifier_decision` and
`test_verifier_repair_execution` exercise these boundaries with synthetic
credentials and real local Git/process fixtures. Stop, scope-revision and
verify-again decisions cannot launch repair through the repair endpoint.
Human-directed rechecks retain the exact prior observation and decision in a v3
request. Each decision permits one child check; dispatch rechecks responder
authority and requires fresh protection evidence. Revoked authority leaves the
prepared claim unconsumed. Candidate, observation, action and responder-kind
substitution refuse in the real-process contract fixtures. Broader
dispatch checks prevent unrelated claims and sibling failed results from bypassing
an unresolved dispute on the same source and candidate, including after service
restart. Repair previews use the same check as dispatch. These checks do not
establish global process containment or stop an agent outside the service. Broader
decision orchestration remains work in progress; these tests do not establish
deployment isolation or real human acceptance.

The loop receipt inbox pins an operator-selected directory descriptor and reads
UUID-named files relative to it, with no-follow and nonblocking open. It rejects
nonregular files before reading and caps reads at 64 KiB. Tests plant a symlink,
FIFO, oversized envelope, injected field, path traversal and replaced directory.
The adapter does not authenticate the envelope: ordinary dispatch still checks
the signed receipt against the pinned issuer and current claim. Configured parent
paths, issuer custody and sandbox deployment remain operator responsibilities.

Automatic-loop enable/pause requests use the reference service's authenticated
session and existing origin controls. They select an existing execution attempt,
not a command, configuration path or issuer. Append-only intent history is checked
before polling; corruption prevents dispatch. Pause is serialized with ticks and
stops future launches, not processes already dispatched. Tests cover authenticated
HTTP configuration, field injection, duplicate intent, background repair/recheck,
pause across scheduler restart and corrupt-history refusal. Intent digests detect
corruption; protected control storage remains required against hostile rewriting.

## Availability observations and queued dispatch (OW-WAR-0109)

The candidate availability adapter runs only an operator-configured command. It
bounds transport time/output, accepts a small exact response schema, and requires
the fresh request nonce. Missing, stale, duplicated, malformed, oversized, timed-out
or unsuccessful observations produce UNKNOWN, never available. Availability is
advisory and grants no authority, writer claim, capacity reservation or assurance.
`test_availability` exercises real subprocesses, including timeout and bad output.
The operator must protect the command and its configuration; the shared process
transport is not a sandbox and cannot contain escaped or remote descendants.
The candidate durable queue records consumption before launch and binds the exact
queue record digest into the execution attempt. A consumed request with no matching
retained attempt stays unknown and is never retried. Restart may recover a single
exact matching attempt. Current configuration identity and shared admission checks
remain necessary. Hashes detect corruption, not hostile rewriting or deletion by
an account that owns control storage. Tests exercise both launch crash boundaries,
restart and corruption. Queue endpoints require the existing authenticated local
session and origin checks. Requests cannot select probe commands or edit server
configuration. Cancellation stops future launch only. Background polling bounds
each tick to one probe and stops on unreadable or corrupt history. Browser lock
does not cancel an already authorized queue request.
## Reference workflow file inputs (OW-WAR-0102)

The shared state/report file reader opens inputs with no-follow and nonblocking
flags, checks the opened descriptor is a regular file before wrapping or reading
it, and closes that descriptor on every exit. Reads retain their byte limit.
`test_file_reads` exercises a FIFO with no writer in a timeout-bounded subprocess,
a directory, and exact/oversized regular-file reads. This prevents the observed
FIFO-open hang and refuses directories without leaking the descriptor. Parent
paths and control storage still require operator protection; this is not a
general filesystem sandbox or a guarantee against slow regular-file I/O.

## Draft candidate review context (OW-ADR-0030)

Candidate review must use the SAS revision named by its captured contract. A
newer file, a different current branch or a Git replacement must not supply a
different governing rule. Candidate snapshots freeze the resolved commit. If the
source file no longer matches its pin, the draft reads regular Git blobs only
from that commit's retained ancestors and checks their full SHA-256 against the
recorded pin. The shared Git reader disables replacement objects and lazy fetching;
it does not run candidate code, hooks or filters. Missing context reports an
unavailable observation rather than substituting the current document.

The acceptance-validity plant advances the SAS after a synthetic bound review;
the baseline falsely rejected that unchanged context. The historical-reader
extension is awaiting a green run. The existing public CLI replacement control
observes refusal of an actually malformed candidate while replacements cannot
change a valid candidate's bytes. These bounded controls do not prove transitive
context closure, private-copy custody or protection of a Git object store from
an account that can rewrite it. Operators must protect retained objects and
control storage. This draft grants no authority and qualifies no live Warrant.


### Experimental stored-review boundary (OW-ADR-0030)

A packet can have a correct canonical digest while omitting required review
inputs. A disposable public CLI probe removed a required binary fixture,
recomputed the packet identity and observed both synthetic verdicts accepted.
Canonical identity alone was therefore insufficient evidence of context closure.

The proposed ingestion control checks every required SAS source, fixture and
gate-evidence source against the reviewed subject's exact digest, and checks
each gate definition's bytes and gate identity. Missing or substituted sources,
duplicate paths, duplicate gate identities and unbound extra sources cannot
supply current verification. Explicitly missing inputs remain missing; this
control does not turn their absence into successful execution evidence.

`rehashed_packets_cannot_omit_or_replace_required_review_sources` exercises
omission, replacement and duplication through the public CLI, followed by exact
restoration. `84-review-source-closure.sh` plants a rehashed missing fixture,
requires `verify.packet-binding` without record/journal writes, then restores
the original packet. The old executable fails the plant by accepting the
incomplete packet. The corrected build passes the plant and all 37 targeted
CLI controls; PR #148 subsequently passed the complete local and hosted gates.
Independent qualification remains pending. This control does not
authenticate the verifier, freeze a hostile same-account workspace, guarantee
complete transitive task context or qualify a production Warrant.

A second disposable probe preserved every required source but substituted an
obligation's statement, then recomputed the canonical packet identity. The
baseline accepted both synthetic verdicts. The task-binding extension compares
each obligation's complete statement, scope and evidence with the caller's
captured contract, checks request assurance and input references, and requires
the exact contract atoms, paths, roles and hashes. Whole-Warrant packets must
contain the whole task; obligation packets must contain one declared obligation.
Duplicate or invented obligations, extra request/obligation/input fields and
unknown packet scopes are refused.
Construction and validation share the atom projection. The candidate reader
passes its captured Git tree, rather than reloading the working checkout.

`rehashed_packets_cannot_replace_the_captured_task` exercises 22 substitutions,
omissions and duplicates with real compiler rehashing, refusal without writes,
and exact restoration. `legitimate_split_task_packets_remain_admissible` keeps
valid per-obligation review and reference-order compatibility supported. All 32
verification CLI tests and CLI all-target Clippy pass on Rust 1.97.1. The source-closure plant also substitutes the task. The acceptance
plant changes a retained packet, record and journal references together in a
disposable Git candidate. The baseline falsely clears that review; the corrected
executable passes all 27 targeted candidate/source-closure controls. Bounded
observations and executable identities are retained in
`docs/tickets/t-b9610/evidence/task-binding-controls.json`. The complete extension
gate and independent qualification remain pending. This is task-field binding,
not verifier authentication or proof that
all rendered evidence excerpts and transitive context are complete and correct.

Unsupported stored verification formats cannot supply current assurance. The
shared SDK decoder keeps an unsupported schema distinct from malformed supported
data. Repository observation reports the unsupported schema as `UNKNOWN`; packet
assembly refuses to silently omit it, and document review retains that diagnostic
instead of claiming the review is absent. Malformed supported records remain
errors. Public CLI controls in `verification_subject_cli.rs` exercise future
versus malformed formats, unchanged records/journal, exact-byte restoration and
document review. These controls neither authenticate a verdict nor adopt v2.

The installed older alpha.2 executable ignores the minimum-reader configuration
and counts changed bare verification records as established. The public CLI
downgrade probe preserves the exact executable hashes and reproduces that
behavior. The draft v2 stored envelope nests the frozen v1 payload, so bare v1
readers must refuse it; current qualification also compares envelope bindings
with the journal and retained packets. Runtime confirmation of that refusal is
pending the current build. This proposal does not protect unchanged legacy
repositories from old executables, establish private custody or authorize a
new schema pack. Pack adoption and preservation of replaced evidence remain
release requirements.


The retention draft preserves the prior active record before replacement using
no-follow descriptor reads and no-overwrite archive publication. Replacement
checks the captured prestate; retention unavailability aborts before any active
replacement. Existing archives are not rewritten. The public CLI history
regression is red on the prior binary; current runtime verification is pending.
The ordinary atomic writer's remaining same-uid race limits still apply.

# Threat model

What OpenWarrant defends, what it does not, and which control carries each
claim. Ten entries; each names the threat, the control, the residual, and what
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
| 1 | **The agent reaches `SSH_AUTH_SOCK`** and signs as the human. Inside an agent session the shell and the human share an ssh agent. | The key is loaded with `ssh-add -c`: every signature asks the human through the agent's confirmation dialog, which no shell can answer. `--ssh-sign` never touches the private key (`ssh-keygen -Y sign` asks the agent). Every signature is a committed artifact (`docs/authority/responses/*.sig`), so a signature made without a dialog is at least visible in history. | `war` cannot verify that `-c` was used; a key loaded without it signs silently. This is the one thing the operator must get right, and the docs say so where the key is configured. Attestations (OW-ADR-0015) make each signature a self-describing artifact but do not close this. | Operator duty. `sign.rs` module doc states it; `--ssh-sign` end to end was measured Allow→"Good signature", Deny→refused on 2026-09-11. |
| 2 | **A pseudo-terminal defeats the TTY check.** `war sign` at a terminal requires `isatty`; an agent can allocate a pty. | Accepted. The TTY path is a guard against the loop being completed by accident, not a cryptographic one; trust rests on the key and the confirmation dialog (entry 1). `signed_via = "tty"` is recorded so a reader can tell the two forms apart. | An agent that deliberately allocates a pty can pass the TTY guard. The remedy is `--ssh-sign` with `ssh-add -c`; the TTY path exists for a human at a real terminal. | Unit test: `at_a_terminal()` is false under `cargo test`; the no-terminal refusal is tested. |
| 3 | **The askpass helper accepts on Deny.** A helper that exits 0 for any button makes the dialog decorative. | The operator's askpass helper parses the agent's `CONFIRM` prompt kind and returns non-zero on Deny; a helper that answers every prompt with 0 is the bug entry 1's dialog exists to prevent. | The helper lives in the operator's environment, not in this repository; `war` sees only "signature verified" or "refused". | Operator's own test of Deny→refused (measured 2026-09-11); the pinentry trap is documented in `docs/authority/allowed_signers.example`. Not exercisable by this repository's gate. |
| 4 | **A newline in the confirmation prompt** lets the prompt carry a second, forged line ("Allow: also sign X"). | The helper receives the prompt with newlines encoded (`%0A`) and renders one line; the thing being signed is shown by `war sign` on the terminal before the dialog, from the record's own facts, never from free text. | Rendering is the helper's. | Operator's helper (the `%0A` rule is in `docs/authority/allowed_signers.example`); `war sign --show` renders the same facts deterministically (unit-tested). |
| 5 | **The agent's environment lacks `SSH_ASKPASS` / `DISPLAY`**, so the dialog never appears and `ssh-keygen` either hangs or fails opaquely. | `war sign --ssh-sign` reports what it observed: the `ssh-keygen` exit status and stderr verbatim in the refusal. It never falls back to a prompt-less path. | A hung dialog is a hung command; the operator kills it. | Refusal path unit-tested; the `sign.ssh-*` rules name the cause; the systemd drop-in need is in `docs/authority/allowed_signers.example`. |
| 6 | **The authority register is edited by the agent** (`docs/authority/roles.toml`, `allowed_signers`): grant itself a role, add its own key. | Both files are human-written by policy and committed: a change is a commit, visible in review, and `war sign` verifies against the committed `allowed_signers`. An agent-kind actor is refused by kind on every act (§27.2, `authorize.agent`, `correction.agent`, …) whatever the register says about it. Duplicate principals are refused (`sign.ssh-principal`). | Policy, not mechanism: nothing stops a commit that edits the register. A branch-protection rule requiring a human review of `docs/authority/**` is the operator's control. | Plants: agent actor refused on authorize/resolve/correct/accept; duplicate principal refused. |
| 7 | **Write-before-confirm.** A response file written to `docs/authority/responses/` before the signature verifies would be a record of an act that never happened. | Verification against `allowed_signers` happens before anything is renamed into place; a signature that does not verify writes nothing. The draft lives beside the final name until then. | None known. | Unit tests in `sign.rs` (verify-before-rename); plant `sign.ssh-verify` refused. |
| 8 | **Same-digest overwrite / archival.** A second response for the same revision could silently replace the first, or an unsent draft could be archived as if sent. | A response for an earlier revision is retired to `<name>.<tag>.response.toml` beside its `.sig`, never deleted; a second retirement onto an existing archive name is refused, not clobbered; "same digest" is distinguished from "no digest found" so nothing unsent is archived. | None known. | Unit tests (`sign.rs`: refused-never-archived, second retirement refused). |
| 9 | **The correction act as a laundering path.** A resolved Warrant's pinned file is edited and a "correction" recorded to make the drift legal. | A correction is a human act (`correction.agent` refuses the agent by kind); it is refused when nothing drifted (`correction.no-drift`), when the new digest is not the file's bytes (`correction.stale`), when the superseded digest is not the chain head (`correction.superseded-mismatch`), when the reason is empty, when the Warrant is not resolved; the journal event carries the record's digest so a later edit of the correction file is `correction.edited`. `deliverables.toml` is never regenerated — the resolution's digest still binds it. | A human can sign a correction whose reason is false. That is a lie in a signed record, which is what the record is for. | Plants `plants.d/64-corrections.sh`: six refusals and the positive `deliverable.corrected`; OBL-005 (superseded digest removed → `correction.malformed`). |
| 10 | **Attestation replay across namespaces.** A signature made for one purpose (a response) presented as another (an attestation), or an attestation for one record presented for another. | Signatures are namespaced: responses sign under `oh.war/response`; attestations sign under `oh.war/dsse` over the DSSE PAE, which binds payload type and length, and the in-toto subjects carry the record digests. A signature under one namespace does not verify under the other (OW-ADR-0015). | An `allowed_signers` line without the `oh.war/dsse` namespace refuses the attestation, not the act; the act stands unattested with `attest.not-emitted`. | Plant `plants.d/67-attest.sh`: a response-namespace signature replayed as DSSE is refused; a moved payload byte, an edited subject and an unknown principal are each refused by name. Unit test in `attest.rs` against a generated key. |

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
`allowed_signers` loader described above. Legacy commands still use those legacy
files; installing this candidate does not silently migrate their trust or make
existing signatures authenticate historical role grants. Production cutover
requires a trusted workflow to consume the new store and refuse legacy fallback.

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

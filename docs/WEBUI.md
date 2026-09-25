# `war ui` — the web UI

```bash
war ui                 # prints http://127.0.0.1:8765/#t=<token>&p=tickets — open it
war ui --port 0        # any free port
war ui --page queue    # open at another page
war ui --as "Ada"      # who signs, when roles.toml names more than one signer
war progress --serve   # the same server, opened at Progress
war ui --lan <addr:port> --cert <pem> --key <pem>   # also paired devices on the LAN (Reach)
```

`war ui` serves one page from the `war` binary on this machine. It is a
**rendering** (SAS §76.6, OW-ADR-0019). Every view is read through the
functions the CLI answers with. Every act it starts is one the CLI would run.
It holds no key.

## Pages

**Tickets** — the ticket loop (docs/TICKETS.md), and the page `war ui` opens
on.
- Every ticket with its state (open, in progress, done), progress (`2/5`),
  priority and who holds a claim, in `war tickets` order.
- Each ticket's checklist as `war show` prints it: done items with who did
  them and their note, open items with the `war claim` that takes them and a
  **ready** badge where `war ready` would offer them, then the notes.
- On this machine's page only: **Claim** on a ready item and **Done** on an
  open one (it asks for an optional note). The server runs the same
  `war claim` / `war done` the CLI does, in process, acting as `--as` when
  given; a refusal (someone else holds the claim, nobody claimed it) comes
  back by its rule. A ticket act is not a signing act and needs no key.
- A paired LAN device sees the same page read-only, with no buttons; the
  server refuses a device's ticket act `act.host-only` (Reach).

**Progress** — the canonical roadmap (OW-ADR-0023).
- The phases in dependency order. Each shows its exit criterion, its tier and
  whether it is achieved.
- Under each phase, its Warrants with rung and how many of §56.1's thirteen
  are unmet.
- The work placed in the phase that no Warrant carries yet.
- The Warrants no phase holds.
- A bar for the whole corpus: resolved, ready to resolve, draft.

**Queue** — every act awaiting a human signature, with its dry-run verdict.
- **would record** — a **Sign** button.
- **needs a decision** — one button per permitted choice, for a resolution's
  outcome or a correction's kind.
- **would be refused** — the refusal's reason, and no button.
- two or more **would record** — also **Sign these N in one dialog**, one
  batch signature over the listed acts (docs/SIGNING.md).

The exact terminal command is always beside the row.

**Questions** — open questions, blocking first, with the asker's
recommendation and the `war answer` that answers each.

**Frontier** — every stage by state (open, claimed, done, blocked) and what
it waits on.

**Corpus** — every Warrant with its rung and unmet count.

**Help** — what next and `war check`'s errors.
- `war next`'s actions, the human ones first. (The ready ticket items
  `war next` lists before them are on Tickets.)
- `war check`'s errors, one row per rule with a count and its remedy (§76.2).
- An *automatic* remedy (never a signing verb, OW-WAR-0112) has a **Run**
  button.

Every page re-reads on its own. The server watches the records' fingerprint
(`war watch`'s, plus the roadmap, the ticket store and its claims). The page asks every two seconds and
refetches when it moves, so a signature given in a terminal appears without
a reload.

## What a button does

**Sign.** A Sign or a choice button asks the server to start one act.
1. The server looks the row id up in an allowlist it built itself, from the
   queue's verdicts. The browser sends an id, never a command.
2. It dry-runs that exact act again. Anything but "would record" stops it,
   and the page says why.
3. Only then does it run `war --root <repo> sign <target> [--outcome …|--kind …]
   [--as …] --ssh-sign`.
4. Your ssh agent's confirm dialog is the signature. Load the key with
   `ssh-add -c`; **without `-c` a click signs without asking**
   (THREAT_MODEL entry 13a).

**Run.** A Run button starts an automatic remedy (`war compile`, `war pins
--refresh …`, `war evidence record …`) the same way.

One act runs at a time. The footer shows it running, and then its exit code
and output.

## What it refuses

| Attempt | Response |
|---|---|
| any `/api/` call without the session token, or with a wrong one | 401 |
| a Host other than the bound `127.0.0.1:<port>` (DNS rebinding) | 400 |
| an Origin other than the page's | 403 |
| an act by any method but POST | 405 |
| an act with no Origin | 403 |
| an act whose id is not on the allowlist | 403, nothing runs |
| an act body with any field but `id` (an argv, say) | 400 |
| a second act while one runs | 409 |
| a ticket act (`POST /api/ticket`) with no Origin | 403 |
| a ticket act naming anything but `claim`/`done` and a `t-…`/`i-…` target, or with another field | 400 |
| a ticket act the ticket commands refuse (claimed by someone else, not claimed) | 409, by its rule |
| headers over 8 KiB, a body over 4 KiB | 431 / 413 |

**The token.** Each start makes a new 256-bit token from the OS random
source.
- It is printed once, in the link's fragment. A fragment is never sent to a
  server or in a Referer.
- The page moves it into memory and clears the fragment.
- It is never in a cookie, in storage or in a request URL.
- A restart rotates it.

**Response headers.** Every response carries:
- a CSP with no inline script or style: `script-src 'self'`,
  `style-src 'self'`, `frame-ancestors 'none'`, `form-action 'none'`;
- `X-Frame-Options: DENY`, `nosniff`, `no-store` and
  `Referrer-Policy: no-referrer`.

**Rendering.** The page writes every value with `textContent`. A record's
text is never parsed as markup.

## Reach

**This machine** is the default, and it does not change: `war ui` binds
127.0.0.1 only. For another device without `--lan`, forward the port over
SSH (`ssh -L 8765:127.0.0.1:8765 host`) and open the link there.

**The LAN** is opt-in, TLS only, and for paired devices only
(OW-WAR-0139).

```bash
war ui --lan 192.168.1.20:8443 --name host.lan --cert host.crt --key host.key
war ui --lan 192.168.1.20:8443 --name host.lan --self-signed   # the fallback
war ui devices                    # the paired devices, and whether each is active
war ui devices --revoke 3fa9c2d1  # its next request is refused
```

- `--lan <addr:port>` binds exactly that address, and nothing binds beyond
  loopback unless you type it — `0.0.0.0` included. The loopback page runs
  beside it, unchanged, and **stays the only page that can sign**.
- It refuses to start without TLS: `ui.lan-needs-tls`, before anything
  binds.
  - `--cert` / `--key`: your certificate, for example from `tailscale cert`
    or a local CA. A device that trusts it shows no warning.
  - `--self-signed`: the fallback. `war` makes a P-256 certificate for
    `--name` and keeps it in `$XDG_STATE_HOME/openwarrant/ui-tls/`
    (mode 0600). Every device shows the browser's warning on its first
    visit. Compare the SHA-256 fingerprint the host prints with the one the
    pairing page shows.
- A plain-HTTP request to the LAN port gets no HTTP answer: the TLS
  handshake fails and nothing is served.
- The Host must be exactly `<name>:<port>` (421 otherwise), and an Origin
  exactly `https://<name>:<port>`. Every response carries HSTS beside the
  loopback page's headers.

**Pairing** is a decision made at the host.
1. The host terminal prints a pairing link, and the same link as a QR
   code. The link carries a single-use code, good for five minutes, and
   the certificate's fingerprint. The code travels in the fragment; the
   page sends it in a POST body.
2. Opening it on the device asks the host's terminal: *pair a device from
   `<address>` (`<user agent>`)? [y/N]*.
3. Only a `y` typed there issues a credential. No answer within 60 s, `n`,
   or no terminal at all issues none: `pair.timed-out`, `pair.declined`,
   `pair.no-tty`.
4. After every attempt, the old code is spent and the host prints a new
   link.

**The credential** is 256 bits from the OS.
- It reaches the device once, as a `__Host-` cookie: `Secure`, `HttpOnly`,
  `SameSite=Strict`. The page's script never sees it.
- The host keeps only its SHA-256, with the device's address, user agent
  and expiry (30 days), in `$XDG_STATE_HOME/openwarrant/ui-devices.json`
  (mode 0600). Nothing about a device is under the repository or in a
  record.
- The file is read on every request, so `war ui devices --revoke <id>`
  takes effect at once. Deleting the file revokes every device.

**On the LAN, nothing is trusted for its address.** Every `/api/` request
needs a paired device's credential, including one from the host's own
loopback address. The loopback page's token means nothing there.

| Attempt on the LAN | Response |
|---|---|
| `--lan` with no certificate and key | refused before binding, `ui.lan-needs-tls` |
| plain HTTP to the LAN port | no HTTP response |
| a Host other than `<name>:<port>` | 421 |
| an Origin other than `https://<name>:<port>` | 403 |
| `/api/` with no credential, a forged one, or the loopback token | 401 |
| a revoked or expired credential | 401, `ui.device-revoked` / `ui.device-expired` |
| a pairing code used twice, or after five minutes | 403, `pair.code-used` / `pair.code-expired` |
| an act without a nonce | 400 |
| an act whose nonce was already spent | 409 `act.nonce-used`, nothing runs |
| a signing act's id | 403 `act.host-only`, nothing runs |
| a ticket act (`POST /api/ticket`), claim or done | 403 `act.host-only`, nothing is written |

**What a device cannot do.** It can never claim or finish a ticket (it
reads them). It can never sign, never start `war sign`,
and never raise the key's dialog on the host. The dialog could be answered
by whoever sits at the host, who may not be the person who clicked.
- Its queue shows each act's dry-run verdict and the exact command to run
  at the host. It carries no act id.
- **Ask at the host** marks the act "requested from `<device>`" in the
  host's queue, and the host terminal prints the command. Nothing starts.
  The signature is still `war sign … --ssh-sign` at the host, and its
  dialog.
- A **Run** button starts an automatic remedy (never a signing verb). Each
  act carries a nonce the server issued to that device, spent on first use.
- A paired device acts for the `--as` signer the server was started with,
  for reading and remedies only.

A stolen device, unlocked, can read the program and run automatic remedies
until you revoke it. It cannot sign. THREAT_MODEL entry 13b has the rest.
Internet reach, port forwarding and relays are not supported: beyond the
LAN, SSH forwarding to loopback stays the answer.

## What it is not

It signs nothing itself. It stores nothing: no drafts, no sessions, no
files — except, with `--lan`, the paired devices' hashes and the fallback
certificate, in the per-user state directory and never under the
repository. It serves no file from disk other than its own three assets. It is
not a projection contract: the `/api/` shapes are local and may change.
`--json` output of the CLI is the contract.

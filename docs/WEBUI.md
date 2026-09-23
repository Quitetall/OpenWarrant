# `war ui` — the web UI

```bash
war ui                 # prints http://127.0.0.1:8765/#t=<token>&p=progress — open it
war ui --port 0        # any free port
war ui --page queue    # open at another page
war ui --as "Ada"      # who signs, when roles.toml names more than one signer
war progress --serve   # the same server, opened at Progress
```

`war ui` serves one page from the `war` binary on this machine. It is a
**rendering** (SAS §76.6, OW-ADR-0019). Every view is read through the
functions the CLI answers with. Every act it starts is one the CLI would run.
It holds no key.

## Pages

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
- `war next`'s actions, the human ones first.
- `war check`'s errors, one row per rule with a count and its remedy (§76.2).
- An *automatic* remedy (never a signing verb, OW-WAR-0112) has a **Run**
  button.

Every page re-reads on its own. The server watches the records' fingerprint
(`war watch`'s, plus the roadmap). The page asks every two seconds and
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
   (THREAT_MODEL entry 13).

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

This machine only: the server binds 127.0.0.1. For another device, forward
the port over SSH (`ssh -L 8765:127.0.0.1:8765 host`) and open the link
there.

The LAN and other devices are on the roadmap as OW-PHASE-9 `web-lan`. That
needs TLS and real authentication before anything that starts an act.

## What it is not

It signs nothing itself. It stores nothing: no drafts, no sessions, no
files. It serves no file from disk other than its own three assets. It is
not a projection contract: the `/api/` shapes are local and may change.
`--json` output of the CLI is the contract.

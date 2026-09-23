---
schema: oh.war/atom/v1
warrant_uuid: 01a0cfbd-745f-7e81-be39-bda3e9378073
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

On 2026-09-23 the owner asked for three things:

- a progress overview page that updates on its own as Warrants move, and
  that reads the canonical roadmap (OW-ADR-0023);
- that page hardened;
- that page part of a main web UI, which should be one of the primary ways
  to use `war`.

Two web surfaces exist and neither is that:

- **`war progress --serve`.** Read-only, loopback, refreshed every few
  seconds, with Host and Origin checks and a CSP. The CSP still allows
  inline script and style. There is no session token. It shows the authored
  `view.json` tree, now built from the roadmap record, and nothing of the
  queue, the questions or the roadmap as a plan.
- **`apps/openwarrant-web`.** A Python reference workbench (OW-WAR-0093,
  0094), which the owner's other checkout has staged for deletion in full.

The owner decided on 2026-09-23:

- **Home.** The web UI lives inside `war`, in Rust, as one binary.
- **Acts.** It may start acts: a button runs the same `war sign <target>
  --ssh-sign` a terminal would. The key's `ssh-add -c` dialog remains the
  human act, and the page holds no key.
- **Reach.** Today it is this machine only. LAN and other devices come
  later.

## Desired Outcome

`war ui` serves the web UI on 127.0.0.1 and prints a one-time URL. `war
progress --serve` becomes the same server opened at its Progress page.

**Progress**, the first page, is the canonical roadmap:

- phases in dependency order, with exit, tier and whether each is achieved;
- under each phase, its Warrants with rung and the thirteen's unmet count;
- the work placed in the phase that no Warrant carries yet;
- the Warrants no phase holds.

It re-reads when the tree changes. The server watches `war watch`'s
fingerprint and the page asks for a version every two seconds, so a
signature given in a terminal shows up without a reload.

**The other pages** are the app's panes, read through the same functions:
Queue, Questions, Frontier, Corpus, Roadmap and Help (what next, with
remedies). Each row shows the exact command behind it.

**Acts from the Queue.** Each signing row shows its dry-run verdict first
and a Sign button only when the verdict is "would record". The button asks
the server to run `war sign <target> --ssh-sign`. An act that needs a
human decision the page cannot take — a resolution's outcome, a
correction's reason — shows its terminal command instead of a button.

**Auto remedies** (OW-WAR-0112's `Kind::Auto`: never a signing verb) can
also run from Help.

Hardened for loopback:

- a per-session token, carried in memory and never in a cookie, URL or
  storage;
- a strict CSP with no inline script or style (assets are separate files
  served by the binary);
- Host and Origin checks, and POST only for acts, with the token and Origin
  both required;
- no CORS; frame-ancestors none; Referrer-Policy no-referrer; request size
  and time limits;
- one act at a time;
- acts limited to an allowlist the server builds from the pending queue and
  the remedy table, never an argv from the browser.

## Non-goals

- LAN or remote reach. That needs TLS and real authentication before
  anything that starts an act, and it is recorded on the roadmap as Phase 9
  `web-lan`, a later Warrant. SSH port forwarding serves until then.
- Authoring Warrants in the browser: the Python workbench's drafting. The
  presets and the app cover authoring; a browser editor is a later Warrant.
- Deciding a resolution's outcome or a correction's reason in the browser.
  Those acts show their terminal command.
- The Python workbench itself. This Warrant neither depends on it nor
  deletes it; its retirement is the other checkout's.

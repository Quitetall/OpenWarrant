---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6063-7762-a36d-143e46efb1d5
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`war ui` (OW-WAR-0116) serves one machine. Its controls are built for
loopback (`webui/mod.rs`, THREAT_MODEL row 13):

- it binds 127.0.0.1;
- the Host must be exactly `127.0.0.1:<port>`;
- the Origin is `http://`, with no TLS;
- a per-start token travels in the URL fragment printed on the owner's
  terminal;
- a Sign button runs `war sign <target> --ssh-sign` on the host, so the
  `ssh-add -c` dialog appears on the host's display.

The owner, 2026-09-23: "Today, just this machine. Eventually, full LAN +
other devices." WEBUI.md and row 13 both say LAN "needs TLS and real
authentication first". None of that exists.

**The hard part is signing, and it does not transfer.** The signing key and
its agent live on the machine running `war`, and so does the confirm
dialog. A phone on the LAN cannot answer that dialog. A button on the phone
that ran the host's `--ssh-sign` would do one of two things:

- raise a dialog in front of whoever sits at the host, who may not be the
  person who clicked;
- wait for a dialog nobody sees.

The first case is worse than refusing, because it can be answered by the
wrong person.

## Desired Outcome

- **Loopback stays the default and does not change.** `war ui` with no new
  flag behaves exactly as today, and `63-webui.sh` still passes.
- **LAN is opt-in and TLS-only.** `war ui --lan <addr:port>` refuses to
  start without a certificate and key. It never serves plain HTTP on the
  LAN listener. In LAN mode the Host must be exactly the configured
  name:port, and the Origin `https://` that name.
- **Devices authenticate by pairing at the host.**
  - The host terminal prints a one-time pairing link (also as a QR code)
    carrying a single-use code and the certificate fingerprint.
  - The host's human confirms each pairing at the host terminal.
  - The device receives a revocable, expiring credential. The server stores
    only its hash, outside the repository, mode 0600.
  - `war ui devices` lists devices; `--revoke` revokes one.
  - In LAN mode no request is trusted because of its source address,
    loopback included.
- **Replay is refused.**
  - A pairing code works once.
  - Every act needs a single-use nonce issued to that device.
  - A revoked or expired credential is 401.
- **Signing stays the human's key dialog, where the key is.** What a LAN
  device may do about a signing act is U-001. Until that is answered, the
  draft is option A:
  - a LAN device sees the queue with each act's dry-run verdict and the
    exact host command;
  - it may run `auto` remedies (never a signing verb);
  - no LAN request starts `war sign`.

## Non-goals

- Access from the internet, port forwarding, or any relay. LAN means a
  network the owner controls; beyond it, SSH forwarding to loopback stays
  the answer.
- A new signature format (WebAuthn or passkeys) verified by
  `authority_check`. That is a change to what a human act is, and it
  belongs to OW-WAR-0138's seam or a later Warrant (U-001 option C).
- Multi-user accounts in the web UI. A paired device acts for the `--as`
  actor the server was started with. Per-person queues come from
  OW-WAR-0137.
- Authoring in the browser.

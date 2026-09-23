---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6063-7762-a36d-143e46efb1d5
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- **OW-WAR-0116** (authorized): the web UI, its controls and its non-goal
  "LAN or remote reach … needs TLS and real authentication before anything
  that starts an act".
- **THREAT_MODEL row 13:** the loopback controls and residual. **Row 1:**
  the `ssh-add -c` dialog is the human act, and it appears where the agent
  runs.
- **§76.6, OW-ADR-0019, OW-ADR-0020.** A view holds no authority. A LAN
  device gets no more authority than the loopback page has.
- **OW-WAR-0138:** what authenticates a human act and what a session is.
  A LAN device credential is a session in that contract's sense. It is not
  an authority to sign.
- **The owner, 2026-09-23:** "Today, just this machine. Eventually, full
  LAN + other devices."

## Assumptions

- A-001: `ssh-keygen -Y sign` through the agent raises the confirm dialog
  on the display the agent's `SSH_ASKPASS` reaches, which is the host's.
  No LAN device sees it. Confidence: high (row 1; measured 2026-09-11).
- A-002: mobile browsers do not let a page pin a certificate. With a
  self-signed certificate, the first visit shows the browser's warning, and
  the fingerprint in the pairing QR code is for the human to compare, not
  the browser. Confidence: high.
- A-003: the hand-rolled server (OW-ADR-0014, no async) can serve TLS
  through a blocking TLS library on the same accept loop. Confidence:
  medium. A TLS stack is a new dependency family, and OW-WAR-0116's
  constraint forbade one (U-002).

## Unknowns

- **U-001 (blocking): what may a LAN device do about a signing act?**
  - **A.** Read and request. The device shows the verdict and the host
    command, and can mark an act "requested from <device>" in the host's
    queue. Signing happens at the host. (Recommended now.)
  - **B.** A remote click raises the host dialog. Refused in this draft,
    because the dialog could be answered by someone other than the person
    who clicked.
  - **C.** A device-held key. The phone signs with its own key (a FIDO
    passkey or an sk key) registered for the same actor. That means a new
    verification path in `authority_check` and a new attestation form
    (RQ-085). It is a later Warrant on OW-WAR-0138's seam.
  - **D.** Signing from the device over SSH to the host, with agent
    forwarding from the device's own agent. Needs no web UI change. It is
    a documented operator path, not a feature.
- **U-002 (blocking): where TLS terminates, and which certificate.**
  - **A.** In `war`, with an operator-supplied certificate (for example
    `tailscale cert` or a local CA). This needs a TLS dependency, and so
    an amendment declaring `Cargo.toml` and `Cargo.lock`.
  - **B.** In `war`, with a generated self-signed certificate and its
    fingerprint shown at pairing. Same dependency, and a browser warning
    on every device.
  - **C.** In an operator reverse proxy (for example Caddy). `war ui`
    stays loopback-only and trusts the proxy through a pinned header
    secret. No new dependency, but the TLS claim becomes the proxy's.

  Recommended: A, with B as a labeled fallback.
- U-003 (non-blocking): where a paired device keeps its credential. The
  draft is a `Secure; HttpOnly; SameSite=Strict` cookie scoped to the LAN
  origin, with Origin checks unchanged. The loopback mode keeps its
  memory-only token.

## Residual risks

- R-001: a paired device that is stolen while unlocked can read the record
  and run auto remedies until it is revoked. It cannot sign under option A.
- R-002: the LAN's other hosts can reach the port. TLS and pairing stop
  them reading or acting. They can still try to connect, and the server
  bounds requests as it does today.
- R-003: pairing confirmed at the host's TTY has row 2's residual. A pty
  defeats the TTY guard. What a pty buys here is a device credential, and
  under option A that credential cannot sign.

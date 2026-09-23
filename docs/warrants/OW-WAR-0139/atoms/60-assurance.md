---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6063-7762-a36d-143e46efb1d5
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — LAN mode serves only TLS, only to the configured name
- **scope:** `war ui --lan` on a scratch program, bound to a
  non-127.0.0.1 address available to the plant: 127.0.0.2, or a network
  namespace's interface where one exists. No claim about any particular
  Wi-Fi network, or about a browser's certificate UI.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - `--lan` without a certificate exits non-zero before binding (no
    listening socket in `ss`);
  - a plain-HTTP request to the LAN port gets no HTTP response: the TLS
    handshake fails, and no body is served;
  - over TLS, a wrong Host is 421 and a foreign Origin is 403;
  - every response carries HSTS and the loopback mode's CSP;
  - the socket is bound only to the given address.

### OBL-002 — an unauthenticated or replaying LAN client gets nothing
- **scope:** the LAN listener and `pairing.rs`, same program.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - an `/api/` request with no device credential is 401. That includes
    requests from the host's own loopback address to the LAN listener;
  - the loopback session token presented to the LAN listener is 401;
  - a pairing code used a second time is refused, and so is one after
    expiry;
  - a revoked credential and an expired credential are each 401;
  - an act replayed with its nonce already used is refused (409), and
    nothing runs twice (the act log has one line);
  - the device file is mode 0600, outside the repository, and holds no
    plaintext credential (a grep for the issued value finds nothing).

### OBL-003 — no LAN request starts a signing act
- **scope:** the allowlist and POST `/api/act` for LAN sessions. U-001
  option A.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - on a program with one pending authorization, the LAN queue row carries
    the verdict and the host command, and no act id;
  - a POST naming the loopback session's signing id from a LAN session is
    403 `act.host-only`, and no process starts (no act log line, no
    `war sign` child);
  - an `auto` remedy runs with a fresh nonce;
  - a source grep finds no key, socket or signing call under `webui/`.

### OBL-004 — pairing is a human decision at the host
- **scope:** `pairing.rs` host confirmation.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a pairing attempt with no answer at the host times out, and no
    credential is issued;
  - an answer of "n" issues none;
  - with no TTY the pairing is refused by name;
  - the confirmation names the device's address.

### OBL-005 — loopback is unchanged
- **scope:** `war ui` without `--lan`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - `63-webui.sh` passes unmodified;
  - loopback responses are byte-identical in headers to the pre-change
    build, apart from the date.

## Gate Adequacy

Required at `controlled`: this Warrant puts the queue of authority acts on
a network. The load-bearing obligation is OBL-003. A LAN click that raised
the host's signing dialog would let whoever sits at the host approve an act
someone else started. That is the confusion row 1's dialog exists to
prevent.

**Adversarial question:** could another host on the LAN act as a paired
device?
- It would need a credential. The server never sends one after pairing
  and stores only its hash.
- A pairing code is single-use and confirmed at the host.
- Even a stolen credential reaches no signing act (OBL-003).

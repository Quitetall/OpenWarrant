---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6063-7762-a36d-143e46efb1d5
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

M1 — LAN mode refuses to exist without TLS:

1. `crates/openwarrant-cli/src/webui/mod.rs`.
   - `--lan <addr:port>` plus certificate and key paths.
   - LAN startup refuses without both, and never binds `0.0.0.0` unless
     the owner types it.
   - In LAN mode:
     - Host is exactly the configured name:port, or 421;
     - Origin is exactly `https://<name>:<port>`;
     - HSTS is sent.
   - Loopback mode is byte-for-byte today's behavior.
2. `crates/openwarrant-cli/src/webui/tls.rs` (new), per U-002.

M2 — pairing and device credentials:

3. `crates/openwarrant-cli/src/webui/pairing.rs` (new).
   - Pairing code: single use, expires after five minutes.
   - Host-TTY confirmation, naming the device's address and user agent.
   - A 256-bit credential. Only its hash is stored in the per-user state
     directory, mode 0600, never under the repository.
   - Expiry, and `war ui devices [--revoke <id>]`.
   - Per-device single-use act nonces.
4. `webui/assets/app.js`: the pairing page, the nonce on each act, and
   host-only rows.

M3 — no LAN request starts a signing act (U-001 option A):

5. `webui/mod.rs` allowlist.
   - For a LAN session, signing rows carry no act id, only the verdict and
     the host command.
   - A POST naming a signing id from a LAN session is 403 `act.host-only`,
     and nothing runs.
   - `auto` remedies stay available with a valid nonce.

M4 — documents and plants:

6. `docs/WEBUI.md`: a Reach section for LAN, pairing, revocation, and what
   a device cannot do.
7. `docs/THREAT_MODEL.md`: row 13 split into loopback and LAN, each with
   its control, residual and plant.
8. `conformance/plants.d/59-webui-lan.sh` (new), the refusals in the
   assurance atom.

## Frozen Surfaces

- Loopback `war ui`: its routes, headers, token handling and
  `63-webui.sh`.
- `war sign` and `--ssh-sign`.
- `oh.war/report/v1`.
- No record schema changes. Device state is local state, not a record.

## Premade Instructions

- Never trust an address. LAN mode authenticates every request, loopback
  included.
- A LAN device never gets an argv or a signing act. If a change makes one
  reachable, it needs a new Warrant (U-001 C).
- Plant each refusal on a scratch program before its positive.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:

- U-001 and U-002, before M1;
- adding any dependency: stop, and amend this Warrant to declare
  `crates/openwarrant-cli/Cargo.toml` and `Cargo.lock` before editing
  them;
- the credential storage choice (U-003).

## Rollback

Remove `--lan`. Loopback is unchanged throughout, so a revert leaves
`war ui` as OW-WAR-0116 delivered it. Revoke every device by deleting the
per-user device file. Nothing under the repository records a device.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0cfbd-745f-7e81-be39-bda3e9378073
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- **SAS §76.6, OW-ADR-0019, OW-ADR-0020.** A view is a rendering and holds no
  authority. The web UI, like the app, issues commands and never signs.
- **OW-ADR-0023.** The roadmap record is canonical. The Progress page is
  built from `roadmap_cmd::view` and `status::build`, never from `view.json`.
- **OW-WAR-0112.** The app's panes and the remedy table, including its
  invariant that no `auto` remedy is a signing act.
- **OW-WAR-0115.** The per-user project list. Once it lands, `war ui` started
  outside a repository offers the same project choice.
- **The owner's decisions, 2026-09-23.** Inside `war`; acts started with the
  key's dialog as the signature; this machine now, LAN later.

## Assumptions

- A-001: `war sign <target> --ssh-sign` needs no terminal (`sign.rs` exempts
  `--ssh-sign` from the terminal gate), so a server child can run it, and
  the `ssh-add -c` confirm dialog is raised by the user's agent, not by the
  page. Verified 2026-09-23 by reading the gate. Confidence: high.
- A-002: a token handed over in the URL fragment (`#t=…`) never reaches the
  server's logs or a Referer header. The page moves it into memory and
  clears the fragment. Confidence: high.
- A-003: other local users can reach 127.0.0.1. The token is what stops
  them; the printed URL is shown only on the owner's terminal.
  Confidence: high.

## Constraints

- One binary, no new runtime and no new dependency family. The HTTP server
  is the existing hand-rolled loopback server in `progress_viewer/server.rs`,
  grown with a POST route and the controls above. It has no async
  (OW-ADR-0014).
- No key, socket or signing call in the web code: a source grep plant, as
  `97-tui.sh` does for the app.
- Every act the page can start is one the CLI would run for the same
  target. The server builds the allowlist; the browser sends only a row id.

## Residual risks

- **R-001: a pty-less signer.** With a key not loaded with `-c`, a click
  signs without a dialog. The same residual applies to `--ssh-sign` from
  any terminal (THREAT_MODEL entry 1). The page states it on the Queue.
- **R-002: a malicious local process holding the token.** Mitigated by
  never writing the token to disk, rotating it per start, and one act at a
  time. The THREAT_MODEL gains a row.

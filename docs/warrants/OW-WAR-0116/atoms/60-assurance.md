---
schema: oh.war/atom/v1
warrant_uuid: 01a0cfbd-745f-7e81-be39-bda3e9378073
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the server refuses everything it should, by name
- **scope:** `webui/mod.rs`, `progress_viewer/server.rs`, loopback only.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** on a scratch program with `war ui --port 0 --no-open`, an API request without the token is 401, with a wrong token 401, with a foreign Host 400, a POST with a foreign or missing Origin 403, a GET on `/api/act` 405, a request over the header bound 431; every response's CSP contains no `unsafe-inline` and includes `frame-ancestors 'none'`; the listening socket is bound to 127.0.0.1 only (from `/proc/net/tcp` or `ss`); a source grep finds no `ssh-keygen`, `SSH_AUTH_SOCK` or signing call under `webui/`.

### OBL-002 — the Progress page is the canonical roadmap, and it stays current
- **scope:** the `progress` view, `app.js`'s Progress page.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `/api/progress` on this corpus lists the eleven phases in the order `war roadmap` prints them, each member's rung equal to `war status --json`'s, the open slugs of the accepted revision, and the unassigned group; no field is read from `view.json`; after a record changes on disk, `/api/version` returns a new value within four seconds and the page's data refetches.

### OBL-003 — a button starts only an act the CLI would run, after its dry run, and a human's dialog signs
- **scope:** POST `/api/act`, the allowlist, the Queue page.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** on a scratch program with one pending authorization, the Queue row carries the dry-run verdict and an allowlist id; a POST naming an id not on the allowlist is 403 and starts no process; a POST with a body carrying an `argv` field is refused 400; the act run for a valid id is exactly `war --root <root> sign <target> --ssh-sign` (asserted from the server's act log line, without a key: the plant expects the child to fail at signing, not to sign); a second POST while one runs is 409; a resolution needing an outcome shows its terminal command and no id.

## Gate Adequacy

Required at `controlled`: the page can start an act. The load-bearing obligation
is OBL-003's "the browser never sends an argv". A server that ran what it was sent
would be a remote shell with a pleasant face.

**Adversarial question:** could a page from another origin make the server sign?
It would need the token, which is in the owner's terminal and the page's
memory only, and an Origin that matches, which a browser does not forge. Even
then the act is the one the server chose, and the key's dialog asks.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0cfbd-745f-7e81-be39-bda3e9378073
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

M1 — the server, hardened:

1. `crates/openwarrant-cli/src/webui/mod.rs`: `war ui [--port] [--no-open]`,
   serving on loopback and printing `http://127.0.0.1:<port>/#t=<token>`.
   - A 256-bit token per start, from the OS random source.
   - Routes: GET `/` and `/assets/*`, embedded with `include_str!`; GET
     `/api/<view>` and GET `/api/version`, both needing the token; POST
     `/api/act`, needing the token and a matching Origin.
   - Every response carries CSP `default-src 'none'; script-src 'self';
     style-src 'self'; connect-src 'self'; img-src 'self'; base-uri 'none';
     form-action 'none'; frame-ancestors 'none'`, plus nosniff,
     no-store and Referrer-Policy no-referrer.
   - Host and Origin are checked, and request line, headers and body are
     bounded.
2. `progress_viewer/server.rs`: the shared request parser and response
   writer, with the inline-allowing CSP removed. `war progress --serve` is
   `war ui` opened at Progress.

M2 — the pages:

3. `webui/assets/{index.html, app.js, app.css}`: pages Progress, Queue,
   Questions, Frontier, Corpus, Roadmap and Help. There is no inline script
   or style. The token lives in memory only, and the version is polled every
   2 s so data refetches on change.
4. The Progress page reads the canonical roadmap and shows:
   - phases in dependency order, with exit, tier and achieved;
   - member Warrants with rung and unmet count;
   - open slugs;
   - the unassigned group.
5. The API views: `progress` (roadmap view with status), `queue` (board acts
   with a dry-run verdict each), `questions`, `frontier`, `corpus`, `help`
   (next actions with remedies). They are read in-process through the
   functions the app uses, and each is cached until the fingerprint moves.

M3 — acts:

6. POST `/api/act {id}` runs one allowlisted act.
   - The allowlist is rebuilt from the pending queue (signing rows whose dry
     run says would-record and that need no decision) and the `auto`
     remedies.
   - It runs as `current_exe --root <root> sign <target> --ssh-sign`, or as
     the remedy's argv.
   - One act runs at a time. Stdout and stderr are returned to the page, and
     the page then refetches.
7. The Queue shows the verdict, a Sign button only when an act is
   allowlisted, and the terminal command otherwise. It also states R-001.

M4 — documents and plants:

8. `docs/WEBUI.md`: every page, every control, the threat notes. `README.md`
   gets a line; `docs/THREAT_MODEL.md` gets a row for R-002.
9. `conformance/plants.d/63-webui.sh`, on a scratch program:
   - no key or signing call under `webui/`;
   - an API request without the token is refused 401;
   - a POST with a foreign Origin is refused 403;
   - a POST naming an id not on the allowlist is refused 403, and nothing
     runs;
   - a GET to `/api/act` is refused 405;
   - responses carry the CSP without `unsafe-inline`;
   - a wrong Host is refused;
   - an oversized request is refused 431;
   - `/api/progress` lists the roadmap's phases in dependency order and a
     member's rung equal to `war status --json`'s;
   - after a record changes, `/api/version` moves within the poll.

## Frozen Surfaces

`oh.war/report/v1`, every record schema, the signing seam. The web API is
local and unversioned for now: it is not a projection contract.

## Premade Instructions

- The browser never sends an argv. It sends a row id; the server looks the
  act up in an allowlist it built itself.
- A page that computes a fact is a second answer. Read the functions the CLI
  answers with.
- Test every act the page starts the way a human would run it: dry run
  first, and only a would-record verdict earns a button.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:

- whether `war` with no arguments offers to open the web UI (draft says no:
  the TUI stays the no-args app, and the TUI's Help names `war ui`);
- the refresh interval (draft: 2 s).

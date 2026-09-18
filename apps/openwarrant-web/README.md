# OpenWarrant reference workbench

A separate workflow application over the OpenWarrant SDK CLI. Python owns HTTP,
local draft storage and browser presentation. The existing Rust SDK owns document
authoring and validation. SDK core imports no web application code. No model or
external service is required. OW93 provides authoring; OW94 adds opt-in execution. Neither establishes Phase 3 exit.

## Run

Requires Linux/macOS, Python 3.11+ and the current built `war` binary:

```sh
cargo build -p openwarrant-cli --bin war
mkdir -m 700 /tmp/ow-workbench
python3 apps/openwarrant-web/server.py \
  --war "$PWD/target/debug/war" --repo "$PWD" \
  --state /tmp/ow-workbench/state --port 8766 \
  --session-file /tmp/ow-workbench/session.json
```

Open the printed loopback URL. Paste the token from the owner-only session file
into the unlock form. The browser keeps it in page memory, not a URL, cookie or
persistent storage. A restart rotates the token; choose a new session-file path.
Stop with Ctrl-C. Keep the state directory to resume drafts. Each revision is a
new checksummed payload file; every editable field and exact source is bound.
The base app exposes no deletion or signing route. Opt-in harness execution and
unverified attempt results are documented in [EXECUTION.md](EXECUTION.md).
Drafts live outside the repository. Download Markdown to inspect or adopt it through
your chosen workflow; saving a draft here does not register a legacy Warrant.

The browser can author title/outcome/scope/context, inspect exact source and older
revisions, and list/search/filter/sort current repository Warrants. Reported work
completion and historical resolution are separate columns. Release reconciliation
routes are proposals, never automatic closure or qualification.

## API

Every `/api/` request needs `Authorization: Bearer <session-token>`. Requests must
use the printed `127.0.0.1:port` Host; browser Origin, when present, must match.
No CORS access is granted. JSON writes require one Content-Length and exactly
`Content-Type: application/json`; transfer encoding and Expect are unsupported.

| Method and route | Result |
| --- | --- |
| GET `/api/warrants` | Latest local draft summaries |
| POST `/api/warrants` | Create one draft from `id`, `title`, `outcome`, `scope`, `context` |
| GET `/api/warrants/{id}` | Latest exact source and editable fields |
| PUT `/api/warrants/{id}` | New immutable revision; same fields plus `expected_source_sha256` |
| GET `/api/warrants/{id}/revisions/{n}` | Exact retained revision |
| GET `/api/board` | Read-only program, objectives, Warrants, stage frontier, open questions and numbered signing commands |
| GET `/api/project` | Live legacy/work-report inventory and proposed reconciliation routes |

The board uses `war board --json`; it never executes the displayed signing
commands. Failed reads clear the board instead of keeping an apparently current
queue. Legacy record status remains separate from implementation completion.

IDs are lowercase UUIDs. Unknown fields, including authority or qualification
claims, refuse. Revision conflicts return 409; reload and review current source.
A lost response does not prove a write failed: read the requested ID before retrying.
Project status uses `war progress --snapshot --json`, sharing the existing
viewer's report and path validation. Malformed reports remain unknown.
The full project read is not an atomic Git snapshot. It reports its input digest
and source report hashes; unavailable reports remain visibly unknown.

## Bounds and trust

The service binds IPv4 loopback only, limits concurrent requests to eight and
closes requests after 15 seconds. Request bodies are limited to 64 KiB; individual
text fields to 16 KB. SDK subprocesses have a 10-second timeout and bounded returned
output. Storage permits 2048 revision records, 1 MiB per record and 64 MiB total.
One process owns a state directory, enforced with an advisory process lock. Writes
use same-directory staging, file sync, no-clobber hard links and directory sync.
Interrupted unpublished staging files are ignored and still count toward the entry
limit. There is no claim of fault-injection qualification or malicious-owner isolation.

The configured SDK executable, repository and private state-directory ancestors
are trusted owner-controlled inputs. This is not a shared-hosting server, sandbox,
OS user boundary, or verified execution harness. Do not expose it through a proxy.
Only existing protected checks and secure human acceptance can support assurance.

## Test

```sh
python3 apps/openwarrant-web/test_workflow.py
```

Tests start real loopback processes and invoke the real SDK CLI. Set `OW_TEST_WAR`
for a non-default build location. They cover restart, source preservation, history,
concurrent stale edits, access refusal, malformed input, store corruption, symlink
refusal and competing process ownership. Browser QA is recorded separately.
The historical aggregate gate remains unchanged because its implementation is a
resolved pinned deliverable; run this package's tests alongside `cargo xtask gate`.

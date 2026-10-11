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
| POST `/api/runtime-assessment` | Read-only reassessment of explicitly selected retained corpus captures; never qualification |
| POST `/api/runtime-captures` | Retain supplied receipt bytes against an actual recorded Dispatch; never assurance |

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

Runtime assessment accepts only `warrant` (a corpus alias) and `selections`, an
array of objects containing `stage_id` and `capture_digest`. For example:

```json
{"warrant":"OW-WAR-0047","selections":[]}
```

An empty selection reports missing evidence; it does not select captures for you.
The service passes a bounded selection request to `war runtime assess` through
stdin. The CLI also accepts `--selection -` and resolution dry runs accept
`--runtime-selection -`; selection files remain supported. Requests allow at
most 256 selections and 16 KiB of serialized input. Returned subprocess output
is limited to 1 MiB per stream, with a 10-second deadline.

Without native policy, native eligibility remains unknown. An operator can set
`--native-store /operator/store` when starting the service. Clients cannot choose
or change that store. The CLI checks its protections and uses fresh native
verification; saved verdicts do not grant authority. The response preserves the
original SDK report and distinguishes `matches`, `unknown` and `refused`, always
with `qualified: false`. Availability failures return 503. This route assesses
existing corpus captures; it does not attach receipts to workbench draft attempts,
activate policy, sign, merge or resolve work. Those workflow connections remain.

Capture ingestion accepts only `warrant` and `capture`. `capture` is the
`oh.war/runtime-capture-inline/v1-draft.1` envelope: `schema`, `request` (the
existing capture-request metadata) and `receipt_base64`. Its request's `receipt`
must be `receipt://sha256:<lowercase hex>` naming the exact supplied bytes.
The SDK checks the recorded Dispatch, compilation event and source basis before
retention. Metadata remains a declaration; importing bytes does not prove native
execution. Explicit assessment performs fresh verification under current policy.

The CLI exposes `war runtime import-inline <alias> --request -`; files are also
supported. SDK callers can use `import_inline` or `import_receipt`. Receipt bytes
are limited to 4 MiB, capture metadata to 16 KiB and the inline envelope to 6 MiB.
Only this HTTP route permits a body of 6 MiB plus 1 KiB of outer framing. Original
file imports retain their path protections. No temporary receipt file is needed.
Exact replay is content-addressed under the same source basis and collector.
Captures remain in history when a new attempt supersedes them; they cannot clear
the new attempt. A lost response leaves retention unknown, not proven absent.
The original SDK report is returned with `qualified: false`; the route cannot
select native policy, activate authority or sign a result.

## Experimental execution context link

An operator can opt into `oh.war/execution-config/v3-draft.1`. Each Warrant
policy then requires a stage plan and an exact `runtime_link`:

```json
{
  "schema": "oh.war/workbench-runtime-link/v1-draft.1",
  "warrant": "IX-WAR-0003",
  "warrant_uuid": "01a0ac9b-3953-7511-86f6-7face232c663",
  "contract_digest": "<the actual 64 lowercase hex characters>",
  "prototype": true,
  "stages": {"build": "STAGE-001"}
}
```

The example is incomplete until the operator supplies the actual digest. The
mapping must cover all workbench stages, with a distinct corpus stage for each.
The browser cannot supply or change this link. Existing v1/v2 execution configs
and request formats are unchanged.

Before launching a worker, the adapter calls the existing SDK CLI to compile and
retain the actual Dispatch, create a portable bundle and validate it. The new
paired `war dispatch --expected-warrant UUID --expected-contract DIGEST` flags
refuse changed identity or contract before writing a Dispatch or compile event.
Matching these fields grants no authority. Contract digests use the existing raw
hex format; bundle digests use the existing `sha256:` prefix.

The worker receives `oh.war/execution-request/v5-draft.1`, including
`runtime_context`: exact original bundle bytes transported as `gzip+base64`,
the bundle digest, retained Dispatch and core attempt identities, and the
workbench identity and source digest. It can validate the bundle offline with
`war dispatch-bundle check --expected-digest`. The adapter hashes the exact bytes
it sends and refuses a change after SDK validation. Raw bundles are limited to
64 MiB; encoded transport to 1 MiB. Run records retain the binding without copying
the large encoded payload.

This first bridge requires repository HEAD and execution worktree HEAD to match.
It does not yet compile a fresh binding for changed resume/repair workspaces or
attach provider receipts automatically to workbench attempts. Known SDK refusal
stops before worker launch; an unavailable or contradictory observation stays
unknown. Portable context does not grant capability permissions, prove sandbox
or spend enforcement, or qualify a result. The current tests use a synthetic
worker and real SDK CLI; they are not production execution qualification.

## Bounds and trust

The service binds IPv4 loopback only, limits concurrent requests to eight and
closes each connection after 15 seconds and uses a three-second socket I/O
timeout. Closing a connection does not cancel an in-flight durable storage
operation. After a lost reply, read current state before retrying a write.
Request bodies are limited to 64 KiB except the bounded capture route; individual
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

Optional local/cloud agent-assisted drafting: [configuration and protocol](DRAFTING.md).

Optional verification candidate: [preparation API and current limits](VERIFICATION.md).

## Next-work preview

Authenticated `GET /api/next-work` evaluates configured whole-Warrant and stage
subjects through the same admission methods used by Start. The browser's Next
work panel shows current ready, blocked, completed-stage or unknown observations.
It cannot grant dispatch permission, reserve a writer, or start work. Start
rechecks current inputs. Failed refresh clears prior entries; locking clears
the panel and discards pending responses from the old session.

This workflow-specific preview is separate from `war progress`'s offline
roadmap. The offline view shows legacy stage facts and cannot establish live
workflow permission without the configured workflow service.

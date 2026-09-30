# Progress viewer adapter

Human progress links open HTML. `war progress --html` writes an offline snapshot
at `.openwarrant/state/progress.html`; an explicit path selects another output.
`war progress --serve` starts a read-only loopback server. `--port 0` chooses an
available port; `--refresh-secs` selects 1–3600 seconds, default 5. Stop with Ctrl-C.
Refresh spacing starts after each completed repository scan; it is not a real-time
latency guarantee. Browser polling adds at most its interval under normal load.

Both modes use the same HTML, embedded data and renderer. No external assets or
model calls. Offline source links refer to the original repository; snapshot data
and UI remain portable. `GET /api/progress` returns `{snapshot,error}`. On refresh
failure, the server keeps its last good snapshot and returns an error; the browser
retains existing content with a stale banner. Requests do not ingest or write data.
Refresh reads new/changed repository records written by their existing owners.

## Work reports

The existing tracker reports legacy resolution, not prototype completion. This
viewer reads an optional `implementation/progress.json` in each configured Warrant
directory. This is a **viewer-local display adapter**, not a new normative SDK
record, authority grant, attestation or lifecycle transition. It is an attributed
claim. Missing/invalid reports show unknown implementation state. A legacy
resolution alone does not mark implementation complete; this viewer cannot grant
or display an established common Verified mark.

```json
{
  "schema": "oh.war/viewer-work-report/v1",
  "warrant": "EXAMPLE-WAR-0001",
  "work_state": "completed",
  "reported_by": "agent://performer",
  "revision": "0000000000000000000000000000000000000000",
  "summary": "Describe observed completed scope.",
  "notes": "docs/warrants/EXAMPLE-WAR-0001/implementation/notes.md",
  "evidence": ["docs/warrants/EXAMPLE-WAR-0001/evidence/check.log"],
  "next_steps": ["Review the exact candidate."]
}
```

Use actual revision and paths, not the example placeholders. States: `in-progress`,
`completed`, `blocked`, `failed`, `cancelled`, `unknown`. `reported_by`, summary,
revision, schema and Warrant are required. Revision is a 40/64-character hexadecimal
source identity label, not proof of trusted execution. For unfinished work it may
name the base; the summary must make that scope clear. Reports are at most 64 KiB;
summary/next-step strings at most 4096 bytes, at most 32 evidence links and 32 steps.
All source paths are relative regular files that resolve within the repository.
Invalid links invalidate that report; failures never become completion.

At a work stop, the performer records observed scope and evidence in this adapter
and regenerates HTML. Human performs no manual metadata step. Completion ratio is
completed reports / all valid explicit reports, never percentage of total code
complete. Unreported Warrant count stays visible. Last scan time, checkout revision
and record digest identify the snapshot; checkout revision does not claim a clean
working tree. Record digest covers the observed tracker/report projection.

## Local service boundary

Bind only 127.0.0.1. Exact Host and same-origin checks defend against cross-site
reads and DNS rebinding. No CORS, POST, arbitrary file route or remote binding.
Evidence routes come from validated report links and return plain text with
nosniff. One bounded connection is handled at a time. Local users with access to
the port can read this view; it is not a multi-user authorization service or
harness sandbox. Do not publish the port through a proxy.

Snapshot export is explicit and atomic. Existing destinations must be regular
viewer snapshots with the generator marker; ordinary files and symlinks refuse.
This operation does not rewrite legacy generated projections or signed sources.

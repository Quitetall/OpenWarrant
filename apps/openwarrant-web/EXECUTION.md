# Reference execution bridge

OW94 adds opt-in execution for SDK-authored drafts. It does not execute legacy
signed Warrants, bypass their gates, qualify results, merge, or publish releases.
A normal start click needs no signature. The owner-configured policy names the
exact permitted source and base commit; draft edits require updating that policy.

## Configure a harness

Pass `--execution-config /absolute/private/execution.json` when starting the server.
Configuration is read once at startup. Browser/API input cannot change it.
A trusted harness owns sandboxing, model routing and remote-job containment. Its
sandbox must restrict writes to the supplied worktree and protect server state,
configuration, repository authority records and evidence. `argv` alone, a skill,
Git worktree, timeout and process groups do not provide that sandbox. Use a tested
harness adapter; the synthetic test process demonstrates the protocol only.

```json
{
  "schema": "oh.war/execution-config/v1",
  "argv": ["/absolute/path/to/trusted-harness-adapter"],
  "sandbox": "harness-worktree-only",
  "cost_mode": "free",
  "spend_limit_usd": 10,
  "timeout_seconds": 1800,
  "repair_cycles": 3,
  "warrants": {
    "00000000-0000-0000-0000-000000000001": {
      "source_sha256": "<64 lowercase hex characters from the saved draft>",
      "base_commit": "<full Git commit ID>",
      "verified_start": false,
      "dependencies": [],
      "checks": [["/absolute/path/to/required-check"]]
    }
  }
}
```

Replace placeholders with real values. Configuration must contain every shown
field. The recommended default cap is $10 and default allowance is three repair
cycles after the initial attempt; the explicit configuration controls both.
`cost_mode: free` is an owner assertion that this harness makes no paid calls.
For a paid or unmetered backend, use `unknown`: a numeric hard cap then refuses
execution. An explicit `spend_limit_usd: null` permits unknown cost, displayed as
unknown rather than zero. This slice does not implement reliable paid metering.

A configured `verified_start: true` refuses because this adapter has no secure
verified-start path. Translate all applicable Warrant start requirements into
owner-reviewed configuration; free text is not an executable permission policy.
Dependencies name other configured draft UUIDs and require completed results for
their exact configured source digests. Every accepted result remains unverified.
No declaration in configuration is itself independent evidence of containment.

## Protocol and public API

`POST /api/runs` takes only `warrant_id` and `source_sha256`; it returns HTTP202
and an attempt record. `GET /api/runs` lists attempts; `GET /api/runs/{attempt_id}`
returns retained state. All use the existing session authentication and local-origin
rules. Lost responses require listing attempts before retrying. Duplicate starts
and starts for already-completed exact source refuse.

The harness receives one JSON object on stdin:

- `schema: oh.war/execution-request/v1`, `attempt_id`, `warrant_id`;
- exact `source`, `source_sha256`, `base_commit`, and isolated `worktree`;
- `qualification: unverified`, `skill: war start`, and declared `limits`.

The harness loads the repository's `war` execution skill and applicable host
instructions, implements the bounded work, and commits its result in that worktree.
The protocol does not invent a standard command for every vendor's agent CLI.
Implement a small adapter around the selected local or cloud agent harness.
Return one JSON object on stdout; diagnostics belong on stderr:

```json
{
  "schema": "oh.war/execution-result/v1",
  "attempt_id": "<exact input attempt ID>",
  "source_sha256": "<exact input digest>",
  "work_state": "completed",
  "notes": "Implemented the declared scope. Evidence is in the committed files.",
  "next_steps": ["Independent review if qualification is needed"]
}
```

Other supported reported outcomes: `blocked`, `failed`. A completed claim requires
successful exit, matching identity, a clean committed worktree descended from its
base, and all configured checks passing without changing that result. These are
performer observations, not independent verification. Required checks and source
constraints remain meaningful only when the configured harness protects them.

One worktree per Warrant is bound in the common Git directory's
`openwarrant-execution` registry. Different server state directories cannot silently
claim that same Warrant. An attempt runs asynchronously with bounded input/output
and a total wall-clock limit across harness and checks. Stdout/stderr capture is
capped at 1 MiB each; retained excerpts are limited to 2048 bytes per stream and include truncation flags.
Configuration is limited to 64 KiB. Maximum256 attempts per store,
maximum20 configured repair cycles, maximum16 required checks.

## Stops, retries and recovery

Attempt records are immutable, checksummed local files under `.execution` in the
private state directory. The initial record precedes dispatch; the final record
preserves actual outcome, check outputs and exact result commit. Draft revisions
remain unchanged. The API/UI projects these records deterministically.

A clean harness-reported blocked/failed result can be explicitly retried within
its configured allowance. No automatic retry is inferred from a lost response.
A timeout, protocol failure or server restart during work leaves execution unknown
and blocks replacement. Process-group termination does not prove escaped writers
or remote jobs stopped. Preserve the original store/worktree; inspect and fence the
old harness before any future recovery adapter permits a replacement. This slice
has no manual "mark stopped" escape hatch and no automatic fencing claim.

Full question routing, resumable unknown-writer recovery, work/harness update
ordering, reports for finer-grained stage stops, secure acceptance and real-agent/user
qualification remain next slices. Attempt-level work reports are documented below. A successful synthetic run is not Phase3 exit.

Run `python3 apps/openwarrant-web/test_execution.py ExecutionTests` alongside the
existing HTTP tests and repository gate. It uses real subprocesses, Git worktrees
and HTTP, with deterministic synthetic harness responses and no model calls.

## Start requirement preview

`POST /api/admission` accepts the same exact `warrant_id` and `source_sha256`
as `POST /api/runs`, using the same authenticated session and body limits.
The browser's **Check start requirements** action displays this read-only snapshot.
It reports the first blocking requirement, `ready`, or `unknown` when inputs or
Git observations are unavailable. It never reserves a writer or grants dispatch.

Start re-runs the shared checks under the execution/authoring lock. Worktree
identity, exclusive writer claim and harness launch remain action-time checks;
a ready preview cannot guarantee them. Qualification stays false. This reference
workflow seam does not unify legacy `next`, `frontier`, `perform` or `console`
admission, and does not establish protected authority or human presence.

## Deterministic work reports

GET `/api/runs/{attempt_id}/report` uses the same authenticated session. It returns
short text, a browser pointer, self-contained offline HTML and a snapshot digest.
Use `--completion-word WORD` (1–64 ASCII letters/digits/underscore/hyphen) and
`--report-detail minimal|full` at service startup. Default word is `WORK_DONE`.
Only a saved final completed/stopped result with a result commit and every exact
required check passing emits that signal. Other reports say `WORK_INCOMPLETE`.
Qualification is always false in this workflow.

The browser's “Work report / offline HTML” action shows short text and offers a
complete HTML download. `/#attempt=UUID` restores that view after session unlock.
Links contain no token. HTML escapes record text and contains no scripts or remote
assets. It retains notes, next steps, checks and source/result identities even when
chat text uses minimal mode. Its counts cover configured execution Warrants only,
not the whole repository. Pending includes configured subjects without a matching
completed result. Snapshot identity binds inventory and observed attempt records.
Reads do not create events; retrying report delivery cannot duplicate completion.
A later service read can show a newer inventory snapshot; downloaded HTML stays
at the recorded snapshot. Reports are performer evidence, not authority or proof
that a malicious process was fenced. Hotline and recovery remain separate work.

## Completion eligibility after policy changes

A retained completed attempt satisfies a current dependency only when its exact
source digest and saved start policy match the configured policy and its required
checks establish completion. The shared predicate also drives progress counts and
the same-subject duplicate-start refusal. Changing required checks, base commit,
dependencies or verified-start requirements makes the old result ineligible for
current work; it does not rewrite the historical attempt. Reports explicitly label
historical results that no longer satisfy current policy. Policy comparison is
conservative: even reordering checks requires a fresh result.

This is deterministic eligibility checking, not authentication of the policy file
or evidence. The configured harness must still protect controller storage and
required checks. Secure human assurance remains separate.

## Wait for a configured agent (OW109)

Opt in with `--availability-config /absolute/private/availability.json`:

```json
{
  "schema": "oh.war/availability-config/v1",
  "argv": ["/absolute/path/to/availability-adapter"],
  "timeout_seconds": 2
}
```

The adapter receives `oh.war/availability-request/v1` with a fresh `nonce`, exact
`subject` and `execution_config_sha256`. It returns only `schema` set to
`oh.war/availability-result/v1`, the same `nonce`, and `state` set to `available`,
`unavailable` or `unknown`. The response limit is 4096 bytes; timeout is 1–5 seconds.
Unknown, invalid and failed observations cannot launch work. This observation is
not a capacity reservation or an authorization; the configured harness still owns
real capacity and isolation. Configure a local, non-billable probe.

`POST /api/queue` takes the same exact subject as direct execution. `GET /api/queue`
lists retained requests and current observations. Dispatched requests include
deterministic `attempt_path` and `report_path` API pointers; unfinished reports
retain their existing refusal/completion rules. `POST /api/queue/cancel` takes
only `queue_id`. All require the existing authenticated local session. The browser
provides Queue saved revision, Refresh agent queue and Cancel queued work controls.
Waiting requests refresh every two seconds while that queue view remains unlocked.

The service polls every two seconds, performs at most one availability probe per
poll and rotates eligible requests. It reuses execution admission and writer
controls. Changed source or execution configuration blocks old requests. Cancel
and enqueue again after reviewing the new configuration. Enqueueing is explicit;
the scheduler does not enroll arbitrary Warrants from the repository.

Requests bind source, optional stage and execution configuration. Consumption is
persisted before launch and the attempt retains its exact queue association. A
restart never repeats a consumed request: an exact retained attempt can be linked,
otherwise the outcome remains unknown and requires investigation. Cancelling a
consumed request refuses, since cancellation cannot stop a process already launched.
Locking the browser does not cancel previously requested work. Scheduler errors
remain visible and stop future automatic dispatch. No queue action awards assurance.

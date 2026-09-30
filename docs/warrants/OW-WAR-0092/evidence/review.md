# Independent review of OW-WAR-0092

Base: b0985fab9e3cc81c18b1cb9771f6f114750bf757. Review used separate contexts and
isolated workspaces. No formal assurance disposition or human acceptance asserted.

## Standards — PASS after fixes

Reviewer /root/viewer_standards_review, workspace /mnt/4tb/tmp/ow92-review-standards.
Two confirmed defects were fixed before publication:

- Export cleaned up a temporary filename even when create_new failed, deleting a
  pre-existing file. Cleanup now starts only after successful creation. Independent
  public CLI recheck pre-created 10,000 filename candidates: command refused with
  File exists, every prior file remained unchanged.
- A cached source replaced with FIFO could block the only serving thread. Safe
  rustix descriptor-relative openat uses NOFOLLOW/NONBLOCK for each component and
  checks the opened descriptor is a regular file. No unsafe application code.

No remaining hard Standards defects or material smell findings in corrected delta.

## Spec — PASS after fixes

Reviewer /root/viewer_spec_review, workspace /mnt/4tb/tmp/ow92-review-spec.
Independent corrected-source probe: FIFO source returns404 and next progress API
returns200 in41ms combined; /etc/passwd symlink returns403 without disclosure;
restored regular notes return200 with exact expected body.

Additional public probes: missing report stays unknown; completed report retains
attribution; script-closing text escapes; traversal invalidates report; ordinary
source overwrite refuses unchanged; refresh0/3601 refuse. No scope creep or
fabricated assurance found. Four committed CLI integration tests cover offline
artifact output, current record refresh, stale retention/recovery, invalid report,
wrong host/origin, POST, traversal, symlink and FIFO cases.

Tested corrected binary SHA256:
9d3c4417f697702d1cf8a201b9dfa714f82d2a829fa2ee635dd6d8fec51c942a.
Dependency declaration subsequently moved from root workspace to CLI-only to keep
an existing unresolved Cargo.toml delivery byte-identical; same rustix1.1.4/fs.

## Browser observations

Live page rendered actual 88-Warrant repository: 2 completed reports, 1 in-progress
report, 85 without a valid work report. Completion and legacy draft/resolved state
are distinct columns; progress denominator is three explicit reports.
Search0076 showed its completed/unverified report; evidence panel exposed source,
notes, gate evidence and next steps. Refresh preserved the expanded panel. Completed
filter retained both0075/0076. Stopping the owned service produced the disconnected/
stale banner while retaining both rows and expanded notes.

The browser tool refused file:// navigation. Offline visual rendering therefore
remains UNKNOWN through that tool; generated offline HTML, embedded JSON escaping,
no external script assets, save/refusal semantics and shared live renderer were
checked. No workaround bypassed the browser URL policy.

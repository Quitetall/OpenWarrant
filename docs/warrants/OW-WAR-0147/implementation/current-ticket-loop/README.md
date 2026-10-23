# Current ticket loop observations

W147's ticket implementation is complete and unverified. These are current Linux
observations at `1f0fa2e34ef608073ac439653dd723ac9609c7b1`, before attachment
of this evidence. No ticket behavior was changed to produce this report.

The existing `45-tickets.sh` plant passes all 20 controls, including the positive
corpus. The selected CLI, claim concurrency, checklist parser and working-form
unit suites pass. Source is clean afterward. The later W148 record kernel
supplies the current implementation and shared `work.*` claim diagnostics; this
does not rewrite historical ticket diagnostics or dispositions.

| Approved obligation | Current observation |
| --- | --- |
| OBL-001 unsigned loop | Create, ready, claim and done work with stdin closed; signing queue unchanged; no authorization, resolution or signature created. Drafting refuses without a drafter and accepts the fixture's exact items. |
| OBL-002 exclusive claims | Concurrent process/thread controls elect one holder. Fresh, stale, stolen and renewed claim cases preserve the named refusals. |
| OBL-003 holder's done | Other-holder, unclaimed and unknown ids refuse. Holder's done changes the checklist and releases its claim. |
| OBL-004 file is state | Hand edits, ordering and byte preservation pass; malformed checklist, duplicate and invalid blocker cases refuse. |
| OBL-005 blockers | All three blocker kinds prevent ready/claim until the actual predecessor completes. |
| OBL-006 opt-in authority | Unsigned tickets check without authority; promotion creates an unsigned draft; working-form manifests and all eight malformed profile variants refuse. |
| OBL-007 prime | Open tasks and notes remain visible; done tasks compact without appearing as pending work. |
| OBL-008 MCP | The live stdio router runs the ticket loop and refuses signing/ingest calls. This is OpenWarrant's MCP, not a model call. |
| OBL-009 latency | Eleven timed commands on this checkout take 24–273 ms, each below 2000 ms. This is machine command time, not human administration or review time. |

The retained logs and [source observation](source-observation.json) identify the
actual inputs, commands and outputs. The population is this current checkout;
the original prose's historical population count and its 5–13 ms measurement
are not relabeled as this run. No reduction or real-user friction target is claimed.

No independent obligation disposition, admissible registered gate, human
acceptance or legacy resolution is created. Historical delivery and judgment
reconciliation remains separate. This report fills a missing implementation
status; it does not convert tests into the common assurance mark.

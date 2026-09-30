---
schema: oh.war/atom/v1
warrant_uuid: 01a0ab92-07b6-7223-b0e6-ad5688b18943
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables
- `apps/openwarrant-web/`: standalone Python standard-library workflow application,
  static browser UI, local HTTP API, and real HTTP integration tests.
- Validated JSON viewer snapshot on the existing CLI (`main.rs`,
  `progress_viewer.rs`, and its integration tests), reusing the canonical validator.
- `docs/releases/`: remaining-Warrant inventory and explicit reconciliation routes.
- This Warrant's implementation notes, progress and observed evidence.

## Required behavior
Bind loopback only. Require a fresh per-process bearer credential for API data and
writes, reject foreign Host/Origin, and never put credentials in URLs or persistent
browser storage. Bound requests, process output, duration, and stored revisions.
Use `war sdk` as the public SDK adapter, with explicit executable/repository paths;
never execute user-supplied shell commands. Documents are draft/unverified only.
Create immutable revision records atomically, compare expected prior digest before
editing, serialize writers, refuse a second process on the state directory, and
preserve earlier versions across restart. The private state directory is trusted
local storage; this is not a sandbox against its owner or a multi-user service.
Show project legacy state separately from reported implementation completion.
Never infer qualification, execution or historical resolution from a draft or UI.

## Acceptance seams
Test real loopback HTTP against the real SDK CLI, including process restart and
concurrent revision conflicts. Browser QA exercises unlock, create, revise, inspect
source, and project status. Invalid inputs, unsupported authority fields, missing
credentials, foreign origins, stale revisions and corrupt storage must refuse.

## Autonomy and rollback
The user's prompt authorizes unverified implementation; human acts stay human.
One isolated worktree, one writer. No paid model calls. Preserve original records.
Stop the service to roll back; its separate state directory never rewrites repo docs.

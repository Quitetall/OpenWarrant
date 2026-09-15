---
schema: oh.war/atom/v1
warrant_uuid: 01a0a2da-52d6-75b0-835e-7cef44cb0a1f
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

CLI --html and --serve options on overview/progress; HTML renderer; read-only
loopback JSON endpoint; tests, viewer-local report documentation and skill pointers.

## Premade Instructions

HTML contains embedded data and no external assets. A live view polls every five
seconds by default, configurable from 1 to 3600 seconds. Show snapshot identity,
last successful refresh, stale/error state, work state, legacy assessment, report
source, evidence/notes and next steps. Never show a percentage of code completion
from unsigned plans or signatures. Preserve last good state when refresh fails.

Serve only loopback; GET only; reject unexpected Host/Origin, oversized requests,
arbitrary filesystem reads and path traversal. Bound connections and report reads.
Offline output replaces only an explicitly chosen non-source destination atomically.
Do not replace existing repository source files or follow destination symlinks.

## Autonomy and rollback

Prompt-only execution and unverified completion permitted. No signing or release
authority is delegated. Paid calls require reliable tracking under the configured
cap (default $10). Default three repair cycles. Preserve original files and history;
stop the local server to disable refresh. No repository writes via HTTP.

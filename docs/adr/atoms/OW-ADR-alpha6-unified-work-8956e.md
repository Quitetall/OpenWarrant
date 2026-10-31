---
schema: oh.war/atom/v1
adr_uuid: 01a1934c-4359-74ce-bfe5-e5c405d27cf6
local_alias: OW-ADR-alpha6-unified-work-8956e
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a19328-8d43-737e-81ca-27e5f18cc1f7"
---

# One Warrant model and a small daily loop

Owner-directed implementation scope, 2026-10-31, under t-8956e. Proposed
format edition changes are not formal acceptance of Format 1.0.

New work uses one Markdown source and keeps its ID while adding stages,
checks and optional authority requirements. One tracker/evaluator and context
interface serves CLI, SDK, MCP and views. All required current checks must
pass; optional checks report results. Completion may remain unverified.
Preserve accepted result versions and history when regressions occur.

Use six daily commands and retain flat compatibility spellings. Derive scoped
claims within existing permissions; shared-checkout overlaps block claims,
isolated worktrees warn about integration. Names identify task/role, never
authority. Cleanup retains unknown, active, foreign or dirty workspaces.
Imports preserve original documents and source identity; changed rows propose
diffs rather than overwriting maintained Warrants.

Keep the existing command runner for custom conditions. Embedded Lua and
cross-repository scheduling are deferred. Legacy signed contracts and frozen
deliverables are not migrated or resigned by this decision.

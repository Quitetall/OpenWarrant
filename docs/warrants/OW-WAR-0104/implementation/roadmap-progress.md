# Visual roadmap implementation progress

2026-09-18. Performer report, unverified. Worktree `codex/visual-roadmap`,
base `8da93652f93cca3d60fd5d8501d3a91cb9d5deb2`; changes not yet committed.

Implemented optional `docs/roadmap/view.json` source, bounded reading, strict
node/reference validation and inclusion in the snapshot digest. The existing
HTML viewer renders a collapsible tree with distinct-Warrant counts, report
states and navigation into the detailed tracker. Unscoped leaf features suppress
completion percentages. Expansion state is retained during page rerendering.

Observed on Rust 1.97.1:
- Parser test passes shared references and refuses duplicate IDs, missing parents,
  parent cycles and unknown Warrants.
- All six progress viewer integration tests pass, including source-only digest
  changes, hostile title escaping and refusal of an unknown Warrant reference.
- In-app browser at isolated local server port 45513 rendered the configured tree:
  release group 15/21, library 10/11, integration 3/4, workflow 2/5, overview 0/1.
  These are attributed implementation counts, not assurance or whole-repository
  completion. The current example does not yet cover every open Warrant.
- Clicking the overview Warrant button set search to OW-WAR-0104 and left its
  matching tracker row plus ancestor roadmap groups visible.

Remaining: complete release inventory, dependency/admission details, richer
feature presentation and phase filtering; automated aggregate/filter tests;
expanded/collapsed refresh, failed-refresh and offline-file visual observations;
full isolated gate and integration. Existing board and legacy reconciliation
obligations remain open. No new assurance or completion claim.

Follow-up: all-target CLI clippy passed before stage-frontier wiring. The six
viewer integration tests passed again after frontier wiring; the added assertion
also confirms successful frontier capture with no hidden error. Stage-state
errors are carried separately and included in snapshot identity. The source
example now names phase 4, shares release Warrants without duplicating aggregate
counts, and the renderer exposes every unmapped repository Warrant. Current
browser observation predates these follow-up UI changes; repeat before closure.

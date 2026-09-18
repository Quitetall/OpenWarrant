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

Calculation follow-up: extracted the exact browser calculations into embedded
`roadmap-model.js`. Three Node built-in tests pass for distinct shared references,
missing/blocked reports, unscoped/empty groups and query/state combinations.
Added a roadmap phase/feature selector. JavaScript syntax check passes.

First isolated gate at e7546c9 exited 1: three steps could not locate the clone's
`target/debug/war`; build used CARGO_TARGET_DIR without the local target link that
legacy corpus/attestation/plant commands require. Earlier build, lint and tests
passed. This is not a full-gate pass; rerun after correcting the clone setup.

Browser follow-up on 946410102a05d6034a67e6204290e7f6c231acc6, local port
33621, in-app browser 2026-09-18:
- Four phases and coverage disclosure rendered: 21 of 106 Warrants mapped,
  85 outside the configured grouping. Release aggregate remains 15/21 despite
  OW90 and OW91 appearing in more than one group.
- Selecting Visual project overview left that feature as the tree root.
- Opening stage details showed OW-WAR-0104 / STAGE-001: open, with the explicit
  legacy-evaluator/no-execution-permission label retained.
- Collapsing the feature and filtering for OW-WAR-0104 retained the collapse
  and displayed exactly its tracker row.
- Generated `/tmp/ow-roadmap-9464101.html`; direct file rendering remains
  unobserved. Earlier file-URL restriction is not bypassed via a local server.

These observations do not cover narrow-screen layout, malformed refresh UI,
full cross-Warrant workflow admission or independent qualification.

Coverage/refusal follow-up: the committed example's authored grouping now covers
all 106 manifest aliases in this checkout. The browser observed live configuration
refresh to 106/106 while retaining the selected feature, collapsed state and
Warrant search. This is coverage, not completion. The uncovered-record disclosure
still handles future additions. A new CLI refusal test corrupts the draft journal:
stage_frontier becomes null, stage_frontier_error contains the journal error,
roadmap stays available and snapshot digest changes. Test passed. OBL-004 is now
included in the existing milestone's obligation list.

The corrected isolated Rust 1.97.1 gate completed successfully at 9464101:
14 steps green, 308 planted controls passed, zero failed. Retained exact log:
`roadmap-gate-9464101.log.gz`. Later coverage/tests/document-link changes are not
covered by that exact revision. Added blocked-only filtering and bounded source
links for governing documents; new traversal refusal test exercises the CLI.
The next-work permission preview remains incomplete: the active reference
workflow evaluator is the application `/api/admission`, which cannot be replaced
by the legacy frontier projection. No permission claim is inferred from open
stages in this viewer.

Workflow next-work implementation: authenticated GET `/api/next-work` calls
existing stage_listing/admission for each configured subject under the executor
lock. Missing inputs produce explicit unknown rows. No worker start or writer
claim occurs. The reference app has a separate refreshable Next work panel,
clears errors/locked state and discards responses from earlier sessions.
All 57 execution HTTP/process tests passed, including preview/admission parity,
no-run/no-claim assertions, wrong-token refusal and verified-start blocking.
JavaScript syntax check passed. New panel browser interaction remains pending.

Next-work browser observation on db7a90cc: disposable authenticated workflow
showed one ready configured subject and zero attempts after unlock. Lock removed
the application panels. The fixture had no corpus configuration, so unrelated
board/project readers refused SDK input; no claim is made about those views.
Fixture service was stopped and its temporary state removed by the test harness.
Subsequent UI improvement adds the source Warrant title and an Inspect action
that loads its saved revision without dispatch. All 57 execution tests passed
again; JavaScript syntax passed. Inspect interaction still needs browser proof.
Hosted run 35370958832 completed successfully for 82e8629 (earlier PR head).

Source-link/refresh control: actual local HTTP test served the exact configured
architecture document bytes. Replacing the roadmap configuration with malformed
JSON retained the prior record digest and returned an explicit Roadmap error.
The focused integration test passed. This proves server behavior; browser stale
labeling is a separate remaining observation for the new roadmap.

Browser stale/recovery proof (2026-09-18): disposable VIEW fixture served on
127.0.0.1:46353. Valid roadmap appeared with one unreported Warrant. After replacing
only its configuration with malformed JSON, Refresh showed “Stale · refresh
failed; last good snapshot retained”, retaining the same record digest. Restoring
valid bytes and refreshing returned “Live · refresh every 1s”. Fixture tab closed
and its owned viewer process stopped. No real repository record was altered.

Owner offline observation received 2026-09-18: “All listed controls work offline”
in response to the request to open `/tmp/ow-roadmap-review.html` directly with
network disconnected and check roadmap, phase selector, search and collapse.
Retained reviewed bytes in `offline-roadmap-reviewed.html.gz`; exact uncompressed
SHA-256 and bounds are in `offline-roadmap-human-observation.json`. This closes
that manual observation gap for those controls and that artifact only. It is
not a signature, assurance disposition, SAS acceptance or Warrant resolution.

Stage-preview test confirms ready API stage and blocked dependent UI stage agree
with direct admission results, preserve the dependency edge, grant no dispatch
permission and create no attempts. All 58 execution tests pass. Three JavaScript
calculation tests pass and are now wired into the reference web CI workflow,
including viewer-source path triggers. Generated check: 960 pass, 88 warnings,
zero unknown and zero errors.

Inspect browser proof on d4c571a4 (2026-09-18): disposable workbench at port
32801 showed “Simpler signup · ready” in Next work. Clicking Inspect loaded
Revision 1 with its exact saved title, outcome, scope and context. The UI still
reported zero attempts. No Save or Start action was invoked. Fixture tab closed;
the test harness stopped the service and removed its disposable state.

Full reference-web test suite passed: 212 tests, 83.113 seconds; exact log retained
as `roadmap-full-web-tests.log.gz`. Read the legacy-question-reconciliation proposal
before treating OW69's old recommendations as owner answers. Several conflict
with later product direction; original question records remain untouched.

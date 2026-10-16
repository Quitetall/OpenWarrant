# Configurable visual roadmap

Draft implementation scope from the owner's 2026-09-18 request. This document
neither accepts the SAS candidate nor declares a Warrant complete.

## Outcome

A developer opens the progress overview and sees what the product will do,
what works now, what blocks the next feature, and what remains before release.
The same configured roadmap produces standalone offline HTML and a locally
refreshed view. Rendering requires no model call or external asset service.

## Source model

A small authored roadmap file describes releases, phases and features. Each node
has a stable ID, title, short outcome, optional parent, display order and explicit
Warrant references. Optional future nodes may have no Warrant yet; display those
as unscoped, never complete. Allow multiple features to reference one Warrant.

The roadmap file owns grouping and intent only. Existing tracker records own
reported implementation state; existing admission evaluation owns whether work
can start. Assurance and legacy resolution remain separate sourced facts.
No editable duplicate status field and no second status database.

Reject duplicate node IDs, missing parents, parent cycles and unknown Warrant
references. Report the offending reference. Do not silently omit invalid nodes.
Dependency edges come from declared work prerequisites, not visual parentage.
Do not infer a scheduling dependency from phase order or layout position.

## First deliverable

- Collapsible release/phase/feature tree, with readable state badges and bars.
- Search and filters for phase, implementation state and blockers.
- Feature detail showing outcome, linked Warrants, blockers, evidence,
  implementation notes, next steps and governing document links.
- Dependency links and a next-work panel backed by the existing admission
  evaluator; preserve refusal and unknown reasons.
- One reproducible renderer for offline snapshots and live local refresh.
- Configuration example and command/API documentation so another repository
  can generate its own roadmap without modifying the renderer.

Board and timeline layouts can reuse this source model later. Interactive edits,
execution controls and signing controls are separate work; this view reads state.

## Counts and uncertainty

Show implementation completion and assurance independently. Label completion
as reported where its source is a performer report. Never map a signed legacy
state to current implementation completion without an explicit supported rule.

For each group, count distinct referenced Warrants across its descendants;
shared references count once. Show numerator and denominator beside percentages. Round percentages down to
whole numbers so unfinished work cannot display 100%.
Unscoped features and unknown/invalid reports remain visible and cannot create
100% completion. Empty groups show no measured completion rather than 100%.
Feature grouping is not an effort estimate. No predicted release date without
an explicit source. Display snapshot revision, generation time and refresh state.

A failed live refresh must not present old state as current. Retain an explicitly
stale snapshot only if the user can clearly distinguish it from current data.

## Proof required

1. Render a fixture with shared Warrants, mixed implementation/assurance states,
   unknown reports, an unscoped feature and explicit blockers. Check exact counts.
2. Refuse cyclic groups, duplicate IDs and dangling references with useful errors.
3. Render hostile source text as text; refuse unsafe navigation URLs and paths.
4. Open the generated HTML without network access. Exercise collapse, search,
   filtering, feature details and source links supported by the offline package.
5. Observe local refresh after a source change and after a failed request.
6. Generate the real OpenWarrant release roadmap from current records; inspect
   desktop and narrow layouts, keyboard navigation and readable status labels.

## Integration boundary

Existing `war board`, progress viewer, tracker and reference web package are reuse
points. OW-WAR-0104 covers the existing read-only board; its unfinished offline
observation and reconciliation remain real requirements. Before implementation,
record this extension in an unsigned Warrant scope or create a successor through
`war new`; do not change signed OW-WAR-0069 atoms or generated projections by hand.

## Current configuration and commands

Place the optional JSON configuration at `docs/roadmap/view.json` relative to the
repository root. The experimental schema is `oh.war/roadmap-view/v1`. `title` names
the map; `nodes` is an ordered array of `id`, `title`, `outcome`, optional `parent`
and `warrants` (an array of local aliases). Array order controls sibling order.
Titles and outcomes are text, not HTML. Parent IDs describe grouping, not work
prerequisites. Unknown fields and references are refused.

Use `war progress --html /tmp/project-progress.html` for a self-contained snapshot,
`war progress --serve --port 0` for the local app's Progress page, or
`war progress --snapshot --json` for the source-backed API envelope. Offline HTML
embeds data and assets; its existing evidence links still require the original
repository. The roadmap does not make an offline evidence archive.

Working-form Warrants appear from their existing manifests, intents and
checklists. Their state is derived by the ticket loop: open, in progress or
done. Claims and checklist edits change the snapshot; intent edits also change
its source digest. No implementation report or signature is required to display
this state. Checklist completion does not award assurance or imply permission
to execute. Directory Warrants retain their separate attributed reports and
legacy assessments.

The JSON overview adds `working_forms` beside the existing legacy fields. Its
counts cover readable records; `coverage_complete: false` and `diagnostics`
explain records that could not be read. `overview --all` includes completed
working forms; the default omits them from `records` while retaining the total
and completion counts. HTML snapshots include all readable working forms,
checklist item counts and links to their source documents. Roadmap references
may name these IDs as well as directory Warrant aliases. Missing source records
cannot silently produce a complete roadmap.

The local app uses the same summary on its Progress page. Its authenticated
`GET /api/progress` response includes `working_forms`; `GET /api/snapshot`
contains the full snapshot. The app retains its existing ticket actions and
human signing controls. Neither a progress read nor a source link signs work.

The tree links into existing detailed Warrant rows. The coverage list exposes
records omitted by the authored map. Group counts deduplicate shared references.
An unscoped leaf suppresses the percentage for its ancestors. Stage dependencies
are projected from the legacy frontier evaluator, and errors remain visible;
these states do not grant current workflow execution permission.

Nodes may also supply `documents`, at most 32 repository-relative file paths.
The viewer validates each path and serves it through the same restricted source
link mechanism as evidence. Parent traversal and files outside the repository
are refused. The reference release node links to the current SAS candidate and
phase plan; these links do not constitute SAS acceptance.

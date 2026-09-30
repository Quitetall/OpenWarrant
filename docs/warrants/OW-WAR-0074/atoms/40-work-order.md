---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e81-7722-b56c-53f89968bd3c
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

- `docs/sas/drafts/1.0.0-rc.2/`: the existing exact candidate remains unchanged unless separately reviewed.
- `docs/design/rc2-adoption-plan.md` and `docs/design/rc2-source-set-acceptance.adr.md`.
- Source-set subject and capture support in the existing core/compiler/CLI crates: `crates/openwarrant-core/src/sas.rs`, a focused source-set module, existing crate exports, and required direct dependencies only.
- CLI proposal, preview, acceptance, status and SAS-pin integration in `crates/openwarrant-cli/src/main.rs`, `sas.rs`, `repo.rs`, `sign.rs`, `check.rs`, and focused supporting modules. Modify only paths necessary for this adoption slice.
- Focused public-interface tests and disposable fixtures under existing crate tests and `conformance/`; scoped documentation for the new proposal options.
- Proposed source-set revision and captured bytes under `docs/sas/revisions/`; acceptance remains the human's act.
- This Warrant's evidence, preserved revision 1, immutable signature subjects, human correction requests, and final verification request.

## Exact implementation contract

Implement the proposed local adoption decision at SHA-256 `7f93d7eb9826678880c6c8548c40cfdc29cec80e5bc91dd2c74ac91f4ace7dbc`.
Its source-set proposal is opt-in. Existing single-document commands, v1 record
semantics, digest domains, requirement IDs, and signed historical subjects remain
compatible. The new v2 subject binds main document, entire manifest and members,
adoption decision, edition, and predecessor. Capture must be bounded, consistent,
and recoverable; refuse unsafe paths, stale inputs, and incomplete captures.
Use the existing canonicalizer and human signing boundary.

The amendment itself does not sign revision 2 or accept RC.2. The retained
historical acceptance literally named `1.0.0` remains unchanged; its RC.1
designation and eventual Stable identity are separate from the new RC.2 subject.

## Frozen surfaces and correction route

Keep original authorization, response, attestation, candidate and accepted-source
bytes recoverable. `authorization.rev-1.toml` retains the original attestation
subject; `revisions/0001/` retains the original authored contract and projections.
Old attestations must remain verifiable after a new authorization is recorded.

The core and CLI SAS modules have resolved-delivery pins (OW-WAR-0058 D-001/D-002
and OW-WAR-0062 D-005). Reinspect all resolved pins before implementation. Within
authorized revision 2, prepare the new code versions in the isolated worktree and
request human corrections over each actual changed pinned artifact. Unresolved
correction requirements remain visible and block governed completion/integration;
never rewrite old deliverable manifests or sign corrections as an agent.

## Work sequence and test seams

1. Prepare the decision and acceptance fixtures; inspect applicable pins and existing v1 tests.
2. Implement through source-set capture and the public proposal/preview/acceptance/status commands, one admitted/refused slice at a time. Test fictional actors in disposable repositories, never accept or sign the real candidate during testing.
3. Run focused tests and required repository checks on the pinned toolchain, preserving actual failures and unknowns. Prepare correction requests and exact resulting source/fixture/build identities.
4. Supply independent verification inputs. The owner verifies/accepts this implementation and signs required corrections, SAS acceptance and Warrant resolution through the applicable process.

Use one isolated worktree for this Warrant and serialize writers. Paid calls
require the owner's recorded budget policy and reliable accounting; no unmetered
model fallback. Three repair cycles are the existing configurable default.

## Scope limits

This is adoption support, not the Phase 1 document/context compiler, batch signing,
TUI, agent orchestration, automatic acceptance, cost-meter implementation, or
Stable publication. Agent choices may adapt internal algorithms while preserving
the exact decision, required behavior and tests. Material changes to those
constraints, wire meanings or authority return for reviewed revision.

## Rollback

Keep prior accepted records and captured versions. Refused operations publish no
accepted revision. Restore unaccepted changes or pursue the approved correction
route; historical evidence is not erased or reinterpreted.

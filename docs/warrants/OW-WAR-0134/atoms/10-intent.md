---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6016-7f51-b81b-3e8ccc3fdc35
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

In this tool, acceptance is a resolution: a human signs `war sign <alias>`
and `resolution.toml` is written. It binds the contract digest, the
assurance-case snapshot, the digest of `deliverables.toml`, the receipts it
relied on, and a `locator`: `HEAD` at ingest, plus which declared paths
were uncommitted (`resolution_cmd.rs::locate`).

Merge comes after that, and the candidate can change in between:
- the target branch moves and the Warrant branch is rebased or merged;
- a conflict is resolved by hand;
- a squash merge produces a commit the locator does not name.

What already catches a change:
- a pinned deliverable whose bytes move is `deliverable.digest-drift`;
- a contract that recompiles differently is `resolution.stale`.

What nothing catches:
- **Every file the resolution does not pin.** Tests, `Cargo.lock`, a
  module the delivered code calls, a gate's fixture. The resolution's
  meaning ("declared deliverables exist at their recorded digests") stays
  literally true while the behaviour the human accepted changes.
- **An unreachable locator.** After a squash or rebase, `war pins` reports
  the locator `UNKNOWN (commit not readable)` and can no longer verify
  the pin from history.
- **The merge candidate itself.** No command compares the tree that was
  accepted with the tree about to land.

The product spec lists "acceptance validity when an already accepted
candidate changes before merging" as a contract still to specify. Q23 sets
the direction: when the target branch changes, re-run affected checks and
independent verification on the resulting candidate; "evidence whose basis
changed cannot be presented as current proof."

## Desired Outcome

- For a resolved Warrant with a locator, `war pins --candidate <rev>`
  reports which paths changed between the accepted tree and `<rev>`, split
  into in-scope and out-of-scope. Scope is the same subject rule OW-WAR-0133
  selects for evidence reuse.
- In-scope change is a finding, `acceptance.candidate-moved`, naming each
  path. It never edits, disputes or annuls the resolution: the resolution
  is still true of the candidate it accepted.
- No locator, or a locator commit git cannot read, is `UNKNOWN`, never
  "unchanged" (Law 15).
- CI runs it on every pull request against the merge candidate, so a
  moved acceptance is seen before merge rather than after.
- What a moved acceptance requires next is the owner's decision (Q-001).
  This Warrant implements the answer; it does not choose it.

## Non-goals

- The assurance mark and the merge-requires-mark policy (Q49). That is
  OW-WAR-0135.
- Evidence reuse rules. OW-WAR-0133 decides the subject; this Warrant reads
  it.
- Dispute or annulment records (§56.4, §56.5). OW-WAR-0136 builds dispute
  for gate invalidation; a moved candidate is not grounds for disputing a
  resolution that was right about what it saw.
- A merge action in `war`. Merging stays git's and the forge's.

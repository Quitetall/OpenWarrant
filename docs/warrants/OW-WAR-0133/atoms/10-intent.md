---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ff5-77b0-a389-10e0350f4b79
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

A recorded gate run counts toward §56.1 requirement 5 while it is bound to
the contract as it compiles now (`evidence.rs::admissibility`). It is bound
to nothing else. So a receipt survives any change the contract digest does
not see:

- **Source bytes.** `war evidence record` binds one subject,
  `contract:sha256:<digest>`. The receipt names no commit, tree or
  deliverable digest. Edit the delivered code after the run and the old
  receipt still reads as a pass on the new code.
- **The deliverable set.** `war check` reports that the contract digest
  covers 8 of §28.5's 17 elements, and `deliverables` is not one of them
  (OW-ADR-0004). A changed `deliverables.toml` leaves every receipt
  admissible.
- **Fixtures.** `gate_cmd.rs` mints every receipt with
  `fixture_digests = []`. A gate whose fixture changed looks identical.

The product spec states the rule and leaves the mechanism open: "Evidence
whose basis changed cannot be presented as current proof" (Q23). It lists
"admissible evidence reuse after source changes" among the engineering
contracts still to specify.

The Dispatch compiler has the matching gap on the context side:

- **Conflicts are never looked for.** `context_select.rs` and
  `compiler/dispatch.rs` both emit `conflicts: []` unconditionally. §33.4
  says equal-precedence conflicts block readiness. An empty list reads as
  "none found" when nothing was checked.
- **Precedence is the compiler's, not the Warrant's.** §33.4 says a WAR
  SHALL declare source precedence. The compiler assigns
  `AuthorizedWarContract` or `InformativeSource` by item kind; no Warrant
  declares anything.
- **Omission already has a rule:** a required item cannot be omitted, and
  every omission carries a reason (`context.rs`). This Warrant keeps that
  rule and pins it with a plant. It does not rewrite it.

## Desired Outcome

- A receipt names what it observed: the contract, the tree it ran over, and
  the bytes of the deliverable set. A run over a dirty tree says so.
- A recorded run is admissible for requirement 5 only while every subject
  the selected rule reads still holds. When one has moved, `war check`
  names it and says "a record, not evidence", as it already does for a
  moved contract.
- A receipt that does not name its source is `UNKNOWN` for reuse, not
  admissible (Law 15). Resolutions already recorded are not touched.
- The Dispatch context manifest no longer claims a conflict check it did
  not make. What counts as a conflict is the owner's call first (Q-002).
- `docs/RESOLVING.md` states the reuse rule and the three compiler rules
  (source, conflict, omission), with the refusal that holds each.

## Non-goals

- Changing what the contract digest covers. That is OW-ADR-0004's, and
  moving it restales every signed contract.
- Re-binding or re-running evidence for resolved Warrants. Their receipts
  are historical and their resolutions keep binding them.
- Gate invalidation (§45). That is the gate side; this is the subject side.
  OW-WAR-0136 owns propagation.
- Acceptance when a candidate changes before merge. That is OW-WAR-0134.
- Semantic conflict detection between prose sources ("these two atoms
  disagree"). No mechanical rule exists, and this Warrant does not invent
  one.

---
schema: oh.war/atom/v1
warrant_uuid: 01a09274-ad79-7414-a583-cdecca67ed90
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

SAS §36.3 gives a blocking unknown a `resolution_requirement`, and §36.2 gives
accepted residual risk a judgment reference. Both describe **risk**: something
that might be false, and what would settle it.

A program also accumulates a different thing. Not a risk — a debt. A decision
that is *required*, that nobody has written yet, and that blocks specific work
until it exists. There is no unknown in it. The answer is not uncertain; the
authority is simply unwritten.

Today that debt can only be carried outside the corpus. A program tracks it in a
hand-maintained table, and the table is right exactly as long as a human keeps
editing it. Nothing derives it, nothing checks it, and nothing notices when a
decision is required by work that has no reservation for it at all.

## Desired outcome

Decision debt is expressible as what it already nearly is: a blocking unknown
whose resolution requirement is a decision record, bound to the work it blocks,
and optionally naming the reservation that will carry it.

Two fields added to `Assumption`, additively: `bound_to`, naming the stage,
milestone, or roadmap node the debt blocks, and `future_ref`, naming the
reserved decision record. `resolution_requirement` already exists per §36.3 and
gains a typed value for the decision case.

The property that makes this worth adding is what the absence of `future_ref`
means. A blocking unknown that requires a decision and names no reservation is
itself a named gap — visible in the corpus rather than dependent on someone
remembering to write a table row.

## Bound and non-goals

Scope: two additive fields on `Assumption`, their validation, their surfacing in
the Gaps view planned in slice D1, and plants.

Out of scope: authoring any decision record; deciding what any program's debts
are; scheduling; any claim that a reserved decision will be written. This Warrant
makes debt expressible and countable. It does not pay it.

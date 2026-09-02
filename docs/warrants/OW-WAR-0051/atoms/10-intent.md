---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1f2d-75b3-8b0e-2a62c2acab59
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Take ADR 0185's two boundary ratchets to zero and run the declared consolidation
waves, so 0185 can reach the completion condition it wrote for itself.

0185 is `accepted` and its gates run in CI and pre-push, but its Completion
section reads *"(open — freezes when both ratchets reach 0 and the declared waves
have run)"*. Neither half is satisfied. This Warrant is the work that closes it.

## What success looks like

`check_module_direction --strict` and `check_module_exposure --strict` both
report zero, from a tree where the josh waves have actually landed — not from a
lowered ceiling. `tools/module-ceilings.json` is a ratchet: raising a number
needs written justification, and a ceiling above the real value is permission to
regress. Reaching zero by editing the ceiling is failure dressed as success.

## What this Warrant must NOT do

**Do not close the `lossless(L1) -> meta(L3)` inversion by moving or publishing
`lamquant-tui`.** ADR 0185 clause 5 is explicit that consolidation is how these
edges dissolve, and its Progress Log records TWO independent arrivals at the
cross-repo answer on 2026-08-26 alone — one extraction into a new repository
that was discarded, and one proposal to publish or relocate the crate that was
stopped before it landed. Both are the same error: moving code between
repositories to close an edge the collapse removes. The answer is attractive and
wrong; expect to want it.

`git filter-repo` is forbidden by clause 5. Use `josh-filter ':prefix=<path>'`.
The reason is stronger than fork continuity: filter-repo rewrites the meta's
history, which destroys OW-WAR-0043 OBL-001 — re-running the migration at its
frozen commit becomes impossible and the migration artifact is retroactively
unverifiable.

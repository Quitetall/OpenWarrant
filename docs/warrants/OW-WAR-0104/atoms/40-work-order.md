---
schema: oh.war/atom/v1
warrant_uuid: 01a0b2a1-a595-7541-b5ae-17edf7d2bf96
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. Read-only `war board`, with text, JSON and standalone offline HTML output.
   Include program, objectives, every Warrant, stage frontier, open questions and
   numbered exact approval commands. Preserve evaluator errors and source states.
2. Authenticated GET `/api/board` in the reference web package and a disclosure-based
   board view. Render source text safely; no signing buttons. Failed refresh clears
   stale board contents. Keep implementation completion separate from legacy state.
3. Conditional batch mode in war-grill covering every open question by Warrant,
   question ID and stage; retain unanswered items and actual responder authority.
4. Positive/refusal tests, browser observations and retained validation notes.

## Frozen surfaces

Existing report envelope, stage dispatch/submission schemas, signing seam, signed
OW-WAR-0069 atoms, resolved deliverable manifests and human authority records.

## Autonomy and rollback

Prompt-authorized unverified implementation. No model calls are needed. Paid calls
require reliable accounting under the user's configurable cap. Keep changes scoped;
revert implementation commits to roll back, preserving evidence and signed history.

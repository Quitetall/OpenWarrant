---
schema: oh.war/atom/v1
warrant_uuid: 01a021a1-4e5e-7fab-b57f-5ff71bea4973
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§98 Phase 0's exit is "real distributions for authoring cost, amendment
types, and failure causes." §94 lists eighteen measures and eight derived
metrics; OW-WAR-0039 delivered the vocabulary and the untracked-work detector.
Nothing has ever been measured with them.

§100 is where this bites. It says OpenWarrant succeeds when it "measurably
reduces" nine things and increases seven others. Every one of those sixteen
claims is comparative, and there is no BEFORE. A system with no baseline cannot
report a reduction; it can only report a number and hope the reader supplies the
comparison.

## Desired Outcome

A recorded baseline taken before any tuning, and a first distribution
measured against it. §94's derived metrics are computed from real events rather
than defined and left empty.

## Scope

§94's measures and derived metrics, §95's untracked-work detection, and
the §100 metrics that depend on a baseline.

## Non-goals

- No tuning. §94 says assurance defaults SHOULD be tuned from measured
  distributions; measuring is this Warrant, tuning is a later one.
- No claim that any §100 metric has improved. A first measurement is a baseline,
  not a trend.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-078` — Partial; §94 governs.
- Discharges §98 Phase 0 exit and §99 criterion 25.

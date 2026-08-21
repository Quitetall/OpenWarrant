---
schema: oh.war/atom/v1
warrant_uuid: 01a021a1-4e5e-7fab-b57f-5ff71bea4973
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

§94, §95, §100, and §98 Phase 0.

## Prerequisites

OW-WAR-0039 resolved (telemetry vocabulary and untracked-work detection).

## Assumptions and Unknowns

- **Evidenced premise.** The measures exist as typed vocabulary with
  tests; `TELEMETRY_MEASURES` and `DERIVED_METRICS` transcribe §94 exactly.
- **Blocking unknown.** Authoring-minutes and interview-count cannot be measured
  without an instrumented authoring session, and every Warrant so far was
  authored by an agent in a transcript nobody timed.
  *Resolution requirement:* one Warrant authored end to end with wall-clock and
  turn counts recorded at the time, not reconstructed afterwards.
- **Accepted residual risk.** A baseline of one is a baseline. It bounds nothing
  and it is still better than a comparative claim with no referent.
  *Consequence if false:* a later "reduction" claim rests on a single noisy
  sample, which is why the obligation below forbids stating one.

## Constraints and Invariants

- **A metric with no baseline is not a metric.** §100's verbs are
  comparative; a number reported without a referent invites the reader to supply
  one.
- **Untracked-work detection SHALL NOT fabricate** a relation after the fact
  without review (§95). The detector already refuses; this must stay true when
  run against a real corpus with real orphans.

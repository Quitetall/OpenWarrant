---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3eac-7bf2-92ec-9fdddfa15045
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

WMD H2 needs authoritative actions that cannot skip legality or produce
ambiguous phase history. A placeholder loop cannot prove startup, active,
recovery, interruption, or capability-gated admission.

## Desired Outcome

WMD commit `765ab4b` supplies bounded action definitions and deterministic
timeline transitions for one H2 kernel, with refusal tests and no renderer/HPR
dependency.

## Scope

WP-007 action admission, fixed-ratio cost validation, startup/active/recovery,
terminal transitions, interruption, process identity, and capacity bounds.

## Non-goals

Spell execution, physics, networking, persistence migration, UI, and human
authorization. Those remain later WMD packages.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-022` - partial: Warrant traces work to SAS and roadmap.
- `WAR-SAS-RQ-044` - partial: action authority is explicit and bounded.
- `WAR-SAS-RQ-051` - partial: claims are bounded to H2 fixtures and tests.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0b30b-e051-70f3-927a-8cc8f202511d
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

- Candidate SAS RC.3 §§14–15: separate verifier context/workspace, bounded repairs,
  evidence-backed rebuttal recheck, human escalation, stale-basis refusal.
- OW-WAR-0094 execution controller and OW-WAR-0098 shared admission checks.
- OW-WAR-0106 internal stages, hotline, explicit resume and immutable evidence.
- Current AGENTS.md: no automatic model review of development commits. This
  Warrant implements an opt-in product feature; test fixtures make no model calls.

Prerequisite: integrate the OW106 execution contract before publishing dependent
runtime changes. Harness owns sandboxing and protected control storage. If required
independence/protection cannot be established, report UNKNOWN and do not qualify.
Exact provider identities and capability evidence must be configured, not supplied
by a browser caller or inferred from model output.

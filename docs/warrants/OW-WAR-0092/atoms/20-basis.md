---
schema: oh.war/atom/v1
warrant_uuid: 01a0a2da-52d6-75b0-835e-7cef44cb0a1f
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Required context

SAS RC.3 work-stop-contract.md Reports and recovery; current AGENTS.md and CONTEXT.md;
CLI overview/status interfaces; published OW-WAR-0076 at b0985fab.
The owner's prompt authorizes this bounded unverified implementation. No additional
verified-start condition applies. Legacy action gates and signed history still bind.

## Implementation model

Use existing live legacy tracker plus a documented viewer-local display adapter
for explicit implementation/progress.json reports. Reports are attributed claims,
not independent evidence or an SDK standard. Missing or invalid reports mean
unknown implementation state. Never infer work completion from legacy resolution.

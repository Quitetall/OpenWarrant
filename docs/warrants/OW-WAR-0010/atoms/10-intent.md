---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd1-b0d3-7be9-a602-9d2144de440d
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

A Warrant's Work Order declares "Autonomy and Escalation" in prose. Nothing reads
it, so nothing can decide whether a choice an executor made was inside its
authority or was an unauthorized amendment. §30.1 distinguishes a local choice
from an auto-authorizable revision from a manual revision; today all three look
identical: text a human might read.

## Desired Outcome

Autonomy is a typed envelope. A change is classified as local, auto-authorized, or requiring manual revision, and the classification is recorded rather than argued.

## Scope

The three amendment classes of §30, ambiguity behaviour (§30.4), and the amendment record of §31.

## Non-goals

- No enforcement against a live agent; that needs Dispatch (OW-WAR-0023) and the Katana seam (OW-WAR-0026).

## SAS and Roadmap Traceability

No §106 requirement maps to this Warrant directly; it is enabling work named in the Production Roadmap.

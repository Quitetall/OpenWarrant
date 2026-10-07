---
schema: oh.war/atom/v1
warrant_uuid: 01a0f502-4941-70a1-a446-e1eb77dff191
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Runtime work must return a real provider receipt that OpenWarrant can match to the exact work it sent. Today the resolver has no connected receipt store and cannot establish the runtime requirement for any Katana or BLUT stage.

One shared Warrant governs the adapter boundary across OpenWarrant, Katana and BLUT. Each project implements its own side; no drifting copies of this contract. OpenWarrant owns dispatch bindings, import observations and resolution assessment. Providers own execution facts, receipt sealing and their authoritative logs or lineage.

Outcome: an attributable receipt from an actual run can satisfy the matching requirement for its own dispatch, contract, stage and attempt. Wrong, stale, missing, unverifiable or incomplete records cannot. This does not itself establish independent assurance or human acceptance.

Out of scope: rewriting provider conversations, copying BLUT lineage into a Warrant, changing signed legacy records, signing as a human, inventing a receipt digest or performing paid calls without reliable accounting.

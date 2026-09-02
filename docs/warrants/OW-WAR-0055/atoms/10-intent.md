---
schema: oh.war/atom/v1
warrant_uuid: 01a06011-b342-78b3-8ba5-ed5c5cd9ba09
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Answer "where are we?" across the whole corpus, from records, for a human
opening a page and for a stateless agent asking what to do next.

Every per-Warrant projection exists — all nine §17.5 views compile — and
`war resolve` computes all thirteen §56.1 requirements from records. Nothing
aggregates any of it. The one corpus-level document, the Warrant Overview,
is a flat table in which every Warrant reads `draft`, and it says so honestly.

## What this delivers, in one sentence

A `CorpusStatus` projection — Release → Objective → Warrant → Milestone → Stage
on the roadmap axis, Release → Requirement on the spec axis, hinged on Warrant
— rendered as canonical JSON and Markdown, drift-checked like every other
generated view, with `next_actionable` computed for an agent.

## The hierarchy is formalised, not invented

Every level already exists as a record:

| Level | Record |
|---|---|
| Release | an accepted SAS revision (one today: `0.1.0-draft.1`; OW-WAR-0058 makes this real) |
| Objective | SAS §98 phase 0–10, named by `roadmap://OW-PHASE-N/<slug>` |
| Warrant | itself |
| Milestone | `M1..Mn` in the milestones atom |
| Stage | `STAGE-nnn`, §47 |
| Requirement | `RQ-xxx` in SAS §106, with the §34.3 status ladder |

This matches SAS §102 decision 8 verbatim: *"Vision → SAS and Roadmap → WAR →
Milestone → Stage/Dispatch → Artifact/Evidence → Resolution."* The vocabulary
avoids three words the SAS already uses with precise meanings — "milestone",
"phase" (which §24 and §98 both use, differently), and "stage".

## What it refuses to do

This repository has refused, in four separate places, to print a number that
looks like progress. That refusal is kept and made structural: every count in
the projection is a **ladder** of named rungs with the strictest first, and no
ratio is ever computed. §34.3's requirement status is the SAS's own ladder —
`unaddressed / claimed / in_progress / satisfied / superseded` — and
`traceability.rs` has modelled it since Phase 1 with no caller. This Warrant is
its first caller.

The headline number is *satisfied*. Today that is zero, and the projection says
zero. The gap between *claimed* (48 of 57 requirements) and *satisfied* (0) is
the single most useful number in the system and is never collapsed.

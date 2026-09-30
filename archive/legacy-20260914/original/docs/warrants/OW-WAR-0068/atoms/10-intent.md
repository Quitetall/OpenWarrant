---
schema: oh.war/atom/v1
warrant_uuid: 01a09482-125b-7703-9b55-354c754ec661
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Matt Pocock's skills (mattpocock/skills at 3cca18b, 2026-09-04) are the best
front half in the field: grilling by frontier rounds, a glossary that cuts
verbosity, one-paragraph ADRs, a spec template, tracer-bullet tickets with
blocking edges, wayfinder maps, two-axis review, seams-first TDD, and a
discipline for writing documents agents read. Their artifacts are prose on an
issue tracker: a closed ticket is a claim. OpenWarrant has the record (signed
authorization, digests, gates, blind verification, corrections) and a weak
front half: an interview nobody runs, no glossary, no frontier view, one
review axis.

## Desired Outcome

A skill pack in this repository that is a strict superset of the promoted
Pocock set, where every skill lands its output in an OpenWarrant record
instead of a tracker: answers in the draft request, terms in a glossary atom
the Dispatch selector carries, specs as v2 proposals, tickets as milestones
with blocking edges and a `war frontier` command, decision maps as
`blocking_unknown` assumptions, reviews with a standards axis beside the
blind verifier. Measured by `war eval`.

## Scope

Skills under `.claude/skills/`, the `war frontier` command, `CONTEXT.md` as a
glossary atom kind, a standards axis for review, and the writing-for-agents
rules applied to every document an agent reads here (no em-dashes, context
pointers, leading words).

## Non-goals

- A skill never signs and never claims a step it did not run.
- No tracker integration in 1.x: the record is the tracker.

## SAS and Roadmap Traceability

`sas://WAR-SAS-RQ-071`, `sas://WAR-SAS-RQ-072`, `sas://WAR-SAS-RQ-077`;
`roadmap://OW-PHASE-2/plan`.

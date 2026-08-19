---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd4-ab15-7fcf-9b6b-25f71a48d7c5
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

This is the distinction the whole specification exists to protect, and OpenWarrant
does not have it at all.

§40 separates a claim, an evidence item, an observation, an inference, a judgment,
and a resolution into six kinds — and §40.7 lists the substitutions that are
PROHIBITED. The most important is that a performer's own report is not evidence
of its own success. Today OpenWarrant has one undifferentiated notion: text in an
assurance atom.

## Desired Outcome

The six epistemic classes are distinct types. A prohibited substitution is unrepresentable or refused. Evidence carries origin, admissibility, and chain of custody.

## Scope

Epistemic classes and prohibited substitutions (§40), evidence origins, admissibility and custody (§41), and the judgment model (§42).

## Non-goals

- No gate runs producing evidence; that is OW-WAR-0020. This defines what evidence IS so that a gate run can produce it.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-052` — Complete; governing section in Basis.

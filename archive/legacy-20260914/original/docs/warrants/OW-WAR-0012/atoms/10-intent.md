---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd2-7d30-7dc6-a3b7-92bb556e3569
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

A Warrant's Basis names its governing sources in prose. Nothing types them, so
nothing distinguishes a specification from a summary of a specification, or a
first-party measurement from a second-hand claim. §33.3 defines trust classes and
§33.4 defines precedence between them precisely because those distinctions decide
which source wins when two disagree.

## Desired Outcome

Context items are typed, carry a trust class, and resolve by declared precedence. A context manifest is digestible, and a summary always names what it summarises (§33.8).

## Scope

Context items and roles (§33.1–§33.2), trust classes and precedence (§33.3–§33.4), completeness (§33.5), the context manifest and its digest (§33.6), and projection (§33.7).

## Non-goals

- No retrieval. Context is declared and resolved locally; fetching remote context is federation work (OW-WAR-0029).

## SAS and Roadmap Traceability

No §106 requirement maps here directly; this is enabling work named in the Production Roadmap.

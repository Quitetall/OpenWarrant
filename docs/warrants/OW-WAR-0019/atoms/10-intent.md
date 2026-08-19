---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd5-a42a-7dc9-819f-8ca614dc87eb
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

There are no gates. A Warrant's acceptance obligations reference proof that does
not exist as an object. §43 defines a Gate Definition as separately governed and
versioned, qualified before use, and BOUND to a subject — which is what stops a
gate command from being a string somebody typed once.

The parent project's corpus is the cautionary case: of 94 declared gates, 23
invoked a tool, script, or crate that was not in the tree. Those were strings,
not gates.

## Desired Outcome

A Gate Definition is a versioned, governed object. A Gate Binding attaches a qualified definition to a subject. An unqualified gate cannot be bound, and an unbound gate cannot be cited by an obligation.

## Scope

Gate Definitions and their lifecycle (§43.2–§43.3), qualification (§43.4), Gate Bindings (§43.5), reusable gates (§43.6), and subject-owned tests (§43.7).

## Non-goals

- No execution. Running a gate is OW-WAR-0020.
- KF owns the Registry (§43.1, RQ-016). This delivers OpenWarrant's schemas and
  CLI support; the authoritative registry is federation work.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-056` — Complete; governing section in Basis.

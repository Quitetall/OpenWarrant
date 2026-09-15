---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-71ab-a5db-0f2c062305af
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

By this point the system can parse a Warrant, lower it to canonical IR, digest
it, and render it. What it cannot do is answer the only question an authorizer
actually asks: **is this Warrant fit to be authorized, and if not, exactly what
is wrong with it?**

Scattering that answer across four commands means the answer is assembled by a
human, differently each time.

## Desired Outcome

`war check <alias>` gives one deterministic, agent-free verdict — a list of
PASS, WARN, and ERROR lines and a readiness conclusion — over the whole Warrant.
It is reproducible: the same Basis yields the same verdict on any run, with no
model in the path.

Running it green over all five of this repository's own Warrants closes the
Phase 1 bootstrap. From that point OpenWarrant is developed through Warrants it
compiles itself, and §93 dogfooding stops being aspirational.

## Scope

The `war check` command, its diagnostic model, its readiness conclusion, and the
Phase 1 conformance subset of §91.

## Non-goals

- No gate execution. A Warrant's acceptance gates are Phase 6; `war check`
  validates the record, not the work.
- No agent involvement of any kind (RQ-074).
- No preflight (§32.7). Readiness in Phase 1 means the record is well-formed,
  not that an actor could execute it.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-074` — `war check` is deterministic and agent-free. Complete.
- `WAR-SAS-RQ-015` — required atom omission fails closed. Complete: OW-WAR-0002
  made the parser refuse; this surfaces the refusal as a verdict a person reads.

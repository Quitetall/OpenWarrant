---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-71ab-a5db-0f2c062305af
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

- SAS §71.7 (`war check`), §76 (CLI ergonomics), §90–§92 (conformance
  philosophy, the core suite, the aggregate gate), §93 (dogfooding), §98 Phase 1.
- Parent Warrant OW-WAR-0001, contract revision 1.

## Context

The SAS's §71.7 example is precise about the output shape, and the precision
matters: PASS, WARN, and ERROR lines followed by a readiness conclusion. A
single boolean would lose the distinction between "this Warrant has an accepted
residual risk" and "this Warrant references a Gate Binding that does not exist."

The corpus this runs against was written before any of the tooling. That is the
point — the parser, the IR, and now the checker have all been developed against
documents shaped by what a Warrant needs to say, not by what was convenient to
implement.

## Prerequisites

- OW-WAR-0002, OW-WAR-0003, and OW-WAR-0004 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** Every check in scope is a pure function of the Basis.
  Verified structurally: no check reads the clock, the network, or the
  environment.
- **Accepted residual risk.** "Ready" in Phase 1 is a statement about the record
  only. §32 defines readiness as including Preflight, which does not exist yet.
  The verdict must therefore not print the word "READY" unqualified, or it will
  be read as a stronger claim than it is.

## Constraints and Invariants

- **Deterministic and agent-free** (RQ-074). No model, no network, no clock.
  Reproducibility here is not a nicety — a checker whose verdict can vary is not
  a control.
- **Unknown is not failure and not pass** (Law 15). A check that cannot be
  performed reports exactly that. It never degrades to ERROR, which would make
  the Warrant look defective, nor to PASS, which would make an unasked question
  look answered. This is the same rule §96.4 states for migrating legacy gate
  results, and it applies to the checker's own output for the same reason.
- **Every ERROR names the file, the rule, and the offending value** (§76.2).
- **Silence on sound state** (§76.3): a fully sound Warrant produces a short
  verdict, not a wall of PASS lines the reader learns to scroll past.

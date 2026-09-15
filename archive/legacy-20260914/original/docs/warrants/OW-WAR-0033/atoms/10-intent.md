---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b8-7a06-b2a4-af9f2db3f485
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§17.5 names nine projections. Two exist: `full_warrant` and `canonical_json`. The other seven — `work_order`, `adr_section`, `adr_overview` as a WAR view, `stage_dispatch`, `assurance_case`, `status`, `audit` — do not, and several are how an actor actually receives work. A caller requesting one gets an explicit unimplemented error, which is honest but not useful.

## Desired Outcome

All nine projections render. A Work Order projection matches Appendix B; an actor gets its Dispatch; a reviewer gets the assurance case alone.

## Scope

The seven unimplemented views of §17.5, and the Work Order shape of Appendix B.

## Non-goals

- No new semantics. Every projection is a view of the existing IR; a projection that needs new data means the IR is incomplete and that belongs in the Warrant that owns it.

## SAS and Roadmap Traceability

No §106 requirement maps here directly; enabling work named in the Production Roadmap.

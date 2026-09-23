---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6068-7151-8462-b9ce53fa3227
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Phase 10 has no Warrant, and its Exit is unowned (OW-WAR-0114's gap table).
The Exit reads: "a contractor Work Order compiles through the unchanged
technical WAR core, and acceptance, invoicing and legal terms live entirely
in the profile."

The code has no profile mechanism a contractor profile could use:

- `Profile` (`openwarrant-core/src/role.rs`) is a closed enum:
  `delivery` and `decision`. Any other name is `UnknownProfile`, and the
  required roles of each profile are hard-coded.
- The SAS layout's `profiles/` directory does not exist.
- Namespaced extension roles (§16.4) already pass validation when optional,
  and they reach the composition with their digests (`lower.rs`). So
  contractor atoms could ride as `contractor.*` extensions. But nothing
  can make them required for one profile, and "unknown required roles fail
  closed" refuses a required `contractor.*` role today.
- The profile name enters `identity.profile` and
  `format_basis.profile_schema_id`, so it is inside every digest.

SAS §98: "Deliver only after separate legal, finance, and QMS decisions."
None exists in this repository.

## Desired Outcome

- **A profile registry, not a third hard-coded variant.** A profile is a
  pinned definition: a name, the core profile it extends, and the
  namespaced roles it requires. `delivery` and `decision` are its first
  two entries, with the same required roles as today. After this seam, a
  profile is added without touching the core (§2.2: "SHALL NOT require
  redesign").
- **The contractor profile** extends `delivery` and requires these
  namespaced roles:
  - `contractor.parties`;
  - `contractor.terms`: authorization scope, compensation, schedule,
    confidentiality and legal terms, by reference;
  - `contractor.acceptance`: acceptance authority and criteria;
  - `contractor.commercial`: invoice and payment relations.

  These are the §22.3 fields, grouped. Invoice, payment and the contractual
  Work Order are references to Knowledge Fabric or finance records, never
  copies (§22.3: link, not replace).
- **Acceptance through the existing act.** Contractor acceptance is the
  existing resolution act by a human-kind actor holding `resolver`. It
  adds no actor role and no new signing path.
- **The Exit, shown.** A contractor Work Order fixture compiles. Its
  technical IR equals the same atoms compiled as `delivery`, except for the
  profile name and the `contractor.*` atoms. And the frozen core modules
  are unchanged between the seam commit and the contractor commit.

## Non-goals

- Legal, finance or QMS content. The fixture's terms are marked "not a
  legal instrument". Real terms wait for the decisions U-001 names.
- Invoicing, payment or any money movement. Those are references only.
- Knowledge Fabric registration of contractual records (Phase 4).
- Other eventual profiles (experiment, investigation, physical test). The
  registry admits them, and this Warrant defines none.

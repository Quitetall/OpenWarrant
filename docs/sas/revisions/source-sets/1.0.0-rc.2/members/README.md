# OpenWarrant SAS 1.0.0-rc.2

**Current edition: consolidated candidate, ready for architecture review and build
planning. Not accepted, implemented, or Stable.** Updated 2026-09-14.

This edition replaces the earlier unaccepted context-focused draft. It consolidates
all Q1–Q53 and C01–C15 decisions, including later answers that supersede earlier
workflow assumptions. The existing governing source and signed records remain
unchanged; [edition history](../README.md) explains the rc.1/old `1.0.0` label.

## Read in this order

1. [SAS](WAR_Software_Architecture_Specification.md): purpose, boundaries, document
   and compiler behavior, optional assurance, later workflows, and release phases.
2. [Format contract](format-contract.md): actual metadata, unit, pointer, condition,
   request, packet, record, and digest rules; [packet schema](schemas/packet.schema.json).
3. [Examples](examples/README.md): minimal Warrant, detailed outcome with ADR/fixtures,
   offline packet, and explicit negative/unknown expectations.
4. [Phase 1 build scope](phase-1-build-scope.md): eleven primitives, existing-code
   reuse map, 56 cases, runnable-driver plan, and Phase 1/2 exit gates.
5. [Decision map](decision-map.md) and [architecture proposal](context-packets.adr.md):
   supersessions, engineering choices, legacy section mapping, and adoption costs.

The [source-set manifest](source-set.json) binds the candidate's normative documents,
schema, and reference fixtures by exact bytes. It is a review manifest, not an
acceptance record or signature. The [validation record](validation.md) reports
what was checked, remaining repository errors, and limits of the evidence.

The [adoption and consolidation plan](../../../design/rc2-adoption-plan.md)
records the canonical-vocabulary decision, signed legacy transition route and
cross-program requests with their actual implementation limits. Definitions in
SAS §3.1 bind; usage advice in §3.2 remains guidance.

## Main decisions now concrete

- Minimal documents are valid before execution readiness. TOML metadata and stable
  Markdown units make them readable and deterministic to parse.
- Captured sources produce a master and exact task projections. Conditions use
  stage/subsystem/path; unknown applicability includes the rule and dependencies.
- Packets carry offline required bytes and enough routing sources to check declared
  coverage. A small entry view is distinct from package transfer size.
- Automated work and delegated governance remain distinct from human-backed
  assurance or signed completion. Later qualification preserves actual history.
- New Warrants may deliver successor versions without rewriting old accepted bytes.
- Phase 1 tests primitives; Phase 2 delivers the supported CLI/compiler and Stable
  gate; Phase 3 begins with Knowledge Fabric Compiler, then workflow clients.

The exact rendering matrix, production schemas/tests, and workflow mechanisms are
implementation deliverables assigned to their phases, not claims already satisfied
by this specification. The mark's public name is a later branding choice; the
baseline's required meaning is specified here.

## Next build slice

**F01: parse a minimal Warrant, preserve exact unit spans, and reject malformed
framing.** Its direct driver and T01–T06 cases are specified in the build scope.
Before governing implementation begins, adopt the exact candidate through the
existing repository process and record any required ADR/migration. No historical
Warrant needs to be falsely closed to make that possible.

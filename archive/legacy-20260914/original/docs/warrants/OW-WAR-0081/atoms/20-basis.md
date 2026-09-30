---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e99-7a13-9e07-5e82d6cf3764
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

The current accepted SAS and existing ADRs govern this draft's preparation.
The implementation target is SAS 1.0.0-rc.2 and its format contract, currently
an unaccepted candidate. The candidate source-set manifest SHA-256 is
`201b707e9657a3c6674ea113a018a34a54b5ab425f37c417c8968af9f4d2cf31`. Candidate feature/case IDs below are prospective scope,
not claims that the active legacy SAS already contains those meanings.

## Required context

- Candidate SAS, format contract, phase-1-build-scope and the exact relevant
  source-set entries: retain binding text and referenced definitions/exceptions.
- Existing module boundaries named in the work order, plus applicable ADRs and
  resolved-delivery pins, re-inspected before implementation.
- Test seam: build_package and check_package in an offline consumer.
- Scoped inventory: F07, T32, T33, T34, T35, T36, T37.

## Prerequisites

Required predecessor outputs: OW-WAR-0080.
Execution needs the applicable governing basis and this Warrant's authorization.
Cross-Warrant prerequisites are manual holds stated in recognized blocking
assumptions in rationale.toml and in the roadmap's planning index. The current
Assumption parser has no external_dependency field and no automatic predecessor
resolver. Status/closure can expose a blocking unknown, but this is not execution
fencing. Current frontier only orders stages inside this Warrant; an OPEN stage
is not permission to start. The authorizer and performer must inspect the holds.
Retire a blocking assumption only on the stated evidence through the applicable
revision process. Do not require unrelated legacy Warrants to be closed.

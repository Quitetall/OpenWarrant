---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1ea8-78c3-a946-d800169df834
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
- Test seam: rc2_probe --all plus public-boundary integration tests and the repository aggregate gate.
- Scoped inventory: F01, F02, F03, F04, F05, F06, F07, F08, F09, F10, F11, T01, T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56.

## Prerequisites

Required predecessor outputs: OW-WAR-0076, OW-WAR-0081, OW-WAR-0082, OW-WAR-0084.
Execution needs the applicable governing basis and this Warrant's authorization.
Cross-Warrant prerequisites are manual holds stated in recognized blocking
assumptions in rationale.toml and in the roadmap's planning index. The current
Assumption parser has no external_dependency field and no automatic predecessor
resolver. Status/closure can expose a blocking unknown, but this is not execution
fencing. Current frontier only orders stages inside this Warrant; an OPEN stage
is not permission to start. The authorizer and performer must inspect the holds.
Retire a blocking assumption only on the stated evidence through the applicable
revision process. Do not require unrelated legacy Warrants to be closed.

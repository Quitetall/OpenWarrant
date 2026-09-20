---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e8c-7ec1-a56f-990a6ac8f353
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

Implementation target: `docs/sas/drafts/1.0.0-rc.3/`, source-set manifest SHA-256
`217bea8600fd34405b0a521454b84bdffd0c06f05a9bd5b15a2b18168e47be7a`. This is an unaccepted candidate selected for unverified
implementation by the owner, not a replacement signature on the legacy SAS.
Existing applicable authority, access limits and signed records remain binding.

## Required context

- RC.3 SAS, format-contract, sdk-contract and phase-1-build-scope; retain exact
  relevant binding units and their dependencies. Footer examples supplement the
  unchanged historical RC.2 fixtures.
- Migration map, applicable ADRs, host AGENTS.md and resolved-delivery pins.
- Test seam: SDK source/reference codecs first; typed provider response against supplied source snapshots in Phase 2.
- Scoped inventory: S03 (Phase 1); T11–T15 provider acquisition/resolution (Phase 2).

## Prerequisites

Required predecessor outputs: OW-WAR-0075.
Only the interfaces/data used by a stage must exist before that dependent stage.
For mixed-phase predecessors, a Phase 1 caller needs their SDK subset, not their
Phase 2 provider qualification. Independent preparation and fixture work continue.
Record exact input revisions when consumed. A missing interface is a work input
gap, not a request for an unrelated signature.

Ordinary in-scope work may start and complete unverified from the owner's prompt.
No blanket SAS adoption, Warrant authorization or human completion signature is
an execution prerequisite for this unsigned plan. Formal adoption, independent
evidence and secure human acceptance remain requirements for the corresponding
Verified claim. Legacy CLI readiness/resolution still reports its legacy gates;
that report does not implement this candidate workflow or redefine completion.

No additional verified-start action gate is declared by this draft.

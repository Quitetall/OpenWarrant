---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1eb1-76e3-8544-e482e6f6b8d7
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

Implementation target: `docs/sas/drafts/1.0.0-rc.3/`, source-set manifest SHA-256
`c5bca5fbe7df30de39d29370cca2ae6c078f94ad0af8be709f386d48164b8509`. This is an unaccepted candidate selected for unverified
implementation by the owner, not a replacement signature on the legacy SAS.
Existing applicable authority, access limits and signed records remain binding.

## Required context

- RC.3 SAS, format-contract, sdk-contract and phase-1-build-scope; retain exact
  relevant binding units and their dependencies. Footer examples supplement the
  unchanged historical RC.2 fixtures.
- Migration map, applicable ADRs, host AGENTS.md and resolved-delivery pins.
- Test seam: interactive authoring adapter calling the same SDK operations as noninteractive CLI.
- Scoped inventory: Phase 2 ergonomic SDK consumer; P2-INTERACTIVE.

## Prerequisites

Required predecessor outputs: OW-WAR-0087.
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

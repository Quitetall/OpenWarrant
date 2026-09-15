---
schema: oh.war/atom/v1
adr_uuid: e7c79d03-c74f-49c4-affe-7cd9dde16a89
local_alias: OW-ADR-0011
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0399d-05b9-7ad0-b8dc-bf1a226fa641"
---

# ADR OW-0011: Report first; block scope and architecture after qualification

## Status

Proposed. Requires human acceptance before it governs work.

## Context

Immediate blocking would turn an unqualified integration into a production
control. Letting every Bonsai signal block would also conflate advisory
leanness with architectural nonconformance.

## Decision

Pilot pull requests emit and upload evidence without blocking merge. After
independent qualification and an explicit human rollout decision, only two
classes block: paths outside the Warrant machine scope and Bonsai error findings
from the architecture-rule allowlist. Leanness and other non-architecture
findings remain visible but advisory.

The canonical Bonsai source is public. Pull-request CI builds the Warrant
adapter from the protected base, materializes the pull-request candidate only as
data, then clones Bonsai at the full commit SHA bound in that candidate's
Warrant scope and runs its locked build on a GitHub-hosted ephemeral runner. It
does not use `pull_request_target`, a private-source credential, or a
pull-request-provided command or executable path. An unavailable candidate,
source, revision, build, executable, or machine result remains `unknown`, never
pass.

A reproducible public source build makes report-phase evidence available to all
protected pull-request origins. Blocking still requires independent
qualification and explicit human rollout; a separately attested distribution or
equivalently isolated trusted execution path remains stronger provenance for a
future high-assurance rollout.

Required administrator-owned GitHub controls before blocking are: one external
review, required code-owner review, resolved threads, no bypass, strict required
checks, Actions SHA pinning, Dependabot security updates, secret scanning, and
push protection. This ADR does not claim those settings are enabled.

## Consequences

CI and PR template can establish report-phase mechanics now. Blocking rollout
remains deliberately incomplete until an external verifier and repository
administrator record the required evidence and activate required protections.

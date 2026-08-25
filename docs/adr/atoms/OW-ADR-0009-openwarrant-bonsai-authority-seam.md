---
schema: oh.war/atom/v1
adr_uuid: 3db11337-26e7-43f9-89f8-d84ead364568
local_alias: OW-ADR-0009
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0399d-05b9-7ad0-b8dc-bf1a226fa641"
---

# ADR OW-0009: OpenWarrant owns work authority; Bonsai owns repository checks

## Status

Proposed. Requires human acceptance before it governs work.

## Context

Both tools can emit useful output, but output without a clear authority boundary
creates a path for a checker to look like an authorizer or for a Warrant to look
like it verified code.

## Decision

OpenWarrant owns Warrant identity, machine scope, evidence binding, external
verification requests, and lifecycle recording. Bonsai remains a generic,
local-first repository checker. OpenWarrant invokes a fixed Bonsai binary and
records its output; Bonsai does not parse, authorize, or resolve Warrants.

## Consequences

The integration lives in `war bonsai check`, not in Bonsai. A result is evidence
only until an external verifier and a human resolver act through OpenWarrant's
existing seams.

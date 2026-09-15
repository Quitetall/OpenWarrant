---
schema: oh.war/atom/v1
adr_uuid: 7f9b3a70-0850-471c-9fa6-a93fb6a2d4c2
local_alias: OW-ADR-0010
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0399d-05b9-7ad0-b8dc-bf1a226fa641"
---

# ADR OW-0010: Bind machine scope and Bonsai evidence to exact bytes

## Status

Proposed. Requires human acceptance before it governs work.

## Context

Prose scope cannot be compared mechanically against a pull-request diff. A
report over an unchecked worktree cannot prove which commit was examined.

## Decision

An optional Warrant-local `scope.toml` declares repository identity, base ref,
policy path and SHA-256, path scopes, and obligation references. Exact sidecar
bytes join the Compilation Basis and therefore move the contract digest. Evidence
records Warrant digest, sidecar digest, base, head, tree, policy digest, Bonsai
binary/version/output, findings, and verdict. Candidate `head` must equal
checked-out `HEAD`.

## Consequences

Older Warrants remain valid without scope. Bonsai-backed checks require a valid
sidecar and return `unknown` when the checker cannot produce machine output.

---
schema: oh.war/atom/v1
adr_uuid: 7a3d9c1e-5b28-4f0a-9e64-2c8b1d7f3a90
local_alias: OW-ADR-0017
role: adr
jurisdiction: bound
order: 30
classification: internal
status: accepted
governs:
  - "war://01a021a2-b570-7f57-85b2-0f8189873d9e"
---

# ADR OW-0017: relicense from AGPL-3.0-or-later to Apache-2.0

## Status

Accepted 2026-09-12 by the repository owner, who ran the relicense through
`scripts/release-1.0-wizard.sh` at the workstation.

## Context

RELICENSING.md set two preconditions and kept them true from the first commit:
every dependency permissive, gated by `cargo deny check licenses` on every gate
run, and the copyright ours to relicense (one author, with contribution terms
written down ahead of any outside contribution). Apache-2.0 carries an express
patent grant, which is worth more to a protocol implementation others
interoperate with than MIT's brevity. The 1.0 release publishes four crates,
and a version published under AGPL-3.0-or-later could never be relicensed
afterwards, so the flip precedes the tag.

## Decision

`license = "Apache-2.0"`, the Apache-2.0 text as LICENSE, a NOTICE, the
`SPDX-License-Identifier` header rewritten in every file the wall allows, the
`deny.toml` exception for our own licence removed. Versions distributed before
2026-09-12 remain available under AGPL-3.0-or-later; nothing is withdrawn.

A file pinned by a RESOLVED Warrant keeps the old identifier until the
correction that frees it, and is listed in RELICENSING-PENDING.txt meanwhile.
`cargo xtask gate` ratchets on that list: the old identifier passes only for a
file it names, so a half-applied relicense is visible rather than silent.

## Consequences

- `release.yml`'s crates job, which refuses any other licence, can publish.
- Each listed file costs one correction, signed with its own reason.
- CONTRIBUTING.md's dual-licence contribution term is moot for new
  contributions and stays as history.

---
schema: oh.war/atom/v1
adr_uuid: 01a0feb9-d5cd-7a83-9fe9-4d6503031de6
local_alias: OW-ADR-0031
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0feb6-ab80-73a5-abcb-7ade9cfaaeaa"
---

# ADR OW-0031: Documents are typed records; a document's type selects its capabilities

## Status

Proposed by the performer under OW-WAR-0148. It governs nothing until the
owner accepts it.

The owner answered that Warrant's Q-001 on 2026-10-02 with (A), refined:

- Liminal owns the generic document substrate: Nodes, Relations, Workspace
  Basis, Jurisdiction, Holders, synchronization, repair and projection
  machinery.
- OpenWarrant owns the WAR domain, as a Liminal-compatible schema and
  projection layer, together with the code that implements it.
- Liminal runs OpenWarrant's code, as a plugin or through `oh.war/liminal-v1`,
  and does not re-implement it.

This is carried by OpenWarrant SAS 1.3.0 (RQ-061, RQ-064, Law 24, §11.3,
§82) and Liminal SAS 0.1.0-proposed.2 (§100). Both are proposed, and neither
is in force until the owner accepts it.

## Liminal mapping

The model is shaped so that it exports to Liminal without translation:

| OpenWarrant | Liminal |
|---|---|
| record | Node |
| relation | Relation |
| record revision | a member of a Workspace Basis |
| who governs a record | Jurisdiction |

## Context

The owner, 2026-10-02: an engineering planning kit on a standalone compiler,
in which "verification and more is a matter of assigning it to that type of
document." `main` already has most of the parts:

- profiles as data (`oh.war/profile/v1`, OW-WAR-0140);
- a working form that is never authorized or verified (OW-WAR-0147);
- a roadmap that is a record with computed achievement (OW-ADR-0023);
- currency by relation (OW-ADR-0022).

What it lacks:
- a name for "what applies to this kind";
- one compiled model that clients share;
- addressable records and typed relations beyond those special cases;
- a binding between a signature and the type it was signed under.

## Decision

Five primitives, and one closed capability set implemented by the kernel.

1. **Record** `{id, type, body, source, revision}`. Identity and revision
   live on the record. A record's revision is the digest of its own bytes.
2. **Relation** `{from, kind, to}`.
   - A closed core set of kinds the kernel computes from: `part_of,
     depends_on, implements, constrains, evaluates, supersedes`, plus the
     rationale edges.
   - Namespaced kinds are carried and drive nothing.
3. **Type (profile)**, as data. It declares its records, the relations it
   allows and requires, its capabilities, its declared states and its
   projections.
4. **Capability**, from a closed set: `structure, links, claims, acceptance,
   evidence, verification, authorization, resolution, stages`.
   - Prerequisites are checked: for example, `verification` needs
     `evidence`, and `resolution` needs `verification` and `authorization`.
   - A type selects capabilities. It cannot define one.
5. **Projection**: a pure function of a selection of the compiled model,
   never a source.

**States.**
- The fixed kernel states are `draft, open, in_progress, done, accepted,
  superseded, authorized, verified, resolved, achieved`. Each is computed or
  authenticated.
- A type may declare states that **refine** a fixed state. A declared state
  holds only while its parent holds, is entered by an authored and journaled
  event, and never satisfies a check its parent does not.

**Authority stays in the kernel.**
- A type requires acts; it never supplies one.
- No profile field creates an act kind, loosens independence, or stands in
  for a signature, an observed run or an independent verdict.

**The signature covers the type.** A manifest written from this decision on
pins its profile file's digest. Manifest bytes are already in the contract
digest, so no existing contract digest moves.

## Options considered

- **Declarable states with free transitions:** rejected. The kernel could no
  longer tell which states needed a human act.
- **Every record family in the kernel** (requirement, risk, decision…):
  rejected. The kernel knows only `obligation` and `item`, the two its
  capabilities compute on. The rest are profile nouns.
- **A profile digest field in `WarIr`:** rejected, because it moves every
  existing contract digest. The pin goes in the manifest instead.

## Consequences

- Per-kind behaviour stops being chosen by profile name. A decision Warrant
  can resolve.
- One compiled model (`oh.war/model/v1`) is the interface the CLI, web UI,
  TUI, MCP and other clients read.
- New document types (roadmap, ticket, later PRD or test plan) are profile
  files plus projections. They need no new compiler variant.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f26-7ba0-a64b-2618b983ba43
role: basis
jurisdiction: authored
order: 20
classification: internal
---


# Basis

## Governing sources

- SAS §101.2 (accepted revisions are immutable), §101.3
  (architecture-changing revisions need an ADR), §101.6 (generated copies
  state the exact accepted revision and digest).
- SAS §34.1 (requirement references), §105 (reference URI forms: only
  `sas://<requirement-id>` today).
- SAS Law 1 (atoms are authored; parents are projections) and Law 7 (all
  normative decisions are ADRs; RQ-020). The section identity and the two
  steps are a decision, so the ADR is a deliverable.
- RQ-022: WARs trace to SAS requirements and Roadmap. Section references
  are the part of that tracing no check covers.
- OW-WAR-0058: the SAS under §101 governance (`sas.rs`, the revision
  record). OW-WAR-0113 and OW-ADR-0022: atomization was set aside there.
- OW-ADR-0021: this Warrant declares `compile.rs` (OW-WAR-0004),
  `check.rs` (OW-WAR-0114), `sas.rs` in the CLI (OW-WAR-0058, resolved)
  and `openwarrant-core/src/lib.rs` (OW-WAR-0114). On authorization it
  becomes their owner.

## What the code does today (read 2026-09-23)

- `crates/openwarrant-core/src/sas.rs`: `SasRevision` pins version,
  source, sha256 and a §106 snapshot; ids are append-only.
- `crates/openwarrant-cli/src/compile.rs`: `sas_normative` writes
  `NORMATIVE.md` and `NORMATIVE.json` from the document in force, with
  its sha256 and the revision it matches.
- `crates/openwarrant-core/src/sections.rs`: fence-aware heading reader.
- `crates/openwarrant-core/src/traceability.rs`: requirement refs only.
- `docs/sas/revisions/`: five recorded revisions (`0.1.0-draft.1` to
  `1.1.0`). Each sha256 matches the document's bytes at one of the five
  commits that touched `docs/sas/WAR_Software_Architecture_Specification.md`
  (checked 2026-09-23), so every revision's bytes are retrievable from
  history in this repository.
- Warrants pin a revision in `authorization.toml` (`sas_revision`); for
  example OW-WAR-0008 and 0046 are pinned to `0.1.0-draft.1`.

## Assumptions

- A-001: a section boundary is a `#` or `## N.` heading at column zero
  outside a fenced block; `###` and deeper stay inside their section.
  Confidence: high; the throwaway split of `1.1.0` gave 125 units that
  join back to the same sha256.
- A-002: reading older SAS bytes from Git from `war check` is local and
  deterministic, as `contract_history.rs` already does for contracts.
  Confidence: high.

## Unknowns

- U-001 (non-blocking): whether the owner wants the section form in §105
  at all. The sixteen amendments already use it. OW-ADR-0028 proposes it;
  authorizing this Warrant adopts the ADR (as OW-ADR-0022 is adopted with
  OW-WAR-0113), and the SAS text follows only by a SAS revision.
- U-002 (non-blocking): the ADR number. OW-ADR-0024 to 0027 are claimed by
  other drafts on 2026-09-23. If 0028 is taken before this work starts,
  the next free number is used and the deliverable is amended.

## Residual risks

- R-001: a SAS revision that renumbers a section breaks every reference to
  it. OW-ADR-0028 makes section numbers stable once accepted, like §106
  ids; the check in this Warrant reports the broken reference, and the
  rule against renumbering is enforced in step two.
- R-002: in a shallow clone the older bytes are absent and currency is
  UNKNOWN for every Warrant pinned to an older revision. That is the
  honest answer (Law 15).

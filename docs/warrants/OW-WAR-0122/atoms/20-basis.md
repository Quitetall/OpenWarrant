---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ee5-7ba2-9dc3-b9c50dcc6ba1
role: basis
jurisdiction: authored
order: 20
classification: internal
---


# Basis

## Governing sources

- SAS §62, §62.1, §62.2, §62.3: the atom source format, structured atoms,
  exact byte preservation, frontmatter validation.
- SAS §16 (role order, extension roles; §16.4 namespaced roles) and §61
  (the manifest).
- SAS §87.2: parsers bound input, reject malformed encodings, preserve
  unknown extensions, and emit typed diagnostics.
- SAS §90 and §91.1–§91.2: a capability is complete only when planted
  violations fail for the intended reason. Tests 4 and 5 (unknown required
  fields fail closed; optional namespaced extensions survive) and 7–9.
- RQ-013: composition is typed, ordered and deterministic.
- OW-ADR-0002 (frontmatter subset) and OW-ADR-0003 (structured atom
  subset): the tool's grammar, and why it is narrower than YAML.
- `docs/design/openwarrant-product-spec.md`, "Engineering contracts still
  to specify": the document schema and grammar, and standard versus tool
  conformance.
- OW-WAR-0114's gap table, which placed this work at
  `roadmap://OW-PHASE-1/document-grammar`.

## What the code does today (read 2026-09-23)

- `crates/openwarrant-core/src/frontmatter.rs`: the restricted reader.
  It refuses anchors, aliases, tags, flow collections, nested mappings,
  block scalars and duplicate keys. It keeps every unknown key, namespaced
  or not.
- `crates/openwarrant-core/src/structured.rs`: the `.yaml` atom reader,
  two levels deep.
- `crates/openwarrant-cli/src/repo.rs` (atom loading): reads
  `jurisdiction` only, defaulting to `authored`. An unparseable header on a
  `.md` atom is `atom.frontmatter`.
- `crates/openwarrant-core/src/sections.rs`: headings outside fences, by
  exact text.
- `crates/openwarrant-core/src/obligation.rs` and `gate.rs`: the
  obligation and `- **gate:**` forms.
- `crates/openwarrant-core/src/manifest.rs`: serde with no
  `deny_unknown_fields`. As read, an unknown manifest key is ignored. No
  plant has observed this yet.
- A throwaway scan of all 549 Markdown atoms declared by the corpus's
  manifests: every non-ADR atom's `schema`, `warrant_uuid`, `role` and
  `order` match its manifest entry. The five ADR atoms carry `adr_uuid` and
  `governs` instead of `warrant_uuid`.

## Assumptions

- A-001: `atom.header` as specified reports no error on the current
  corpus, because the scan above found no mismatch outside ADR atoms.
  Confidence: high. The first plant run confirms it; if it does not, the
  mismatches are reported and escalated, not fixed by editing bound atoms.
- A-002: the grammar the SAS in force governs is the atom format, not the
  RC.3 document format, because SAS 1.1.0 is accepted and RC.3 is a draft.
  Confidence: medium. See U-001.

## Unknowns

- U-001 (**blocking**): which format this grammar governs. This Warrant is
  written for the SAS 1.1.0 atom format. If the owner wants the RC.3
  document format covered instead, or both, the deliverables change.
  Resolution: the owner confirms the scope before authorization.
- U-002 (non-blocking): whether §62.3's "unknown required fields fail"
  requires refusing an unknown non-namespaced header key or manifest key.
  The document records the tool's current behaviour (kept or ignored) as
  a divergence and proposes SAS wording. It adds no rule.
- U-003 (non-blocking): whether the obligation grammar belongs in the
  standard or only in this tool. The document places it under tool
  conformance and says so.

## Residual risks

- R-001: a grammar document that nobody re-reads drifts from the tool.
  The plants bind each listed construct to the tool's behaviour, so a
  change to either fails the battery.
- R-002: a later Warrant widens a reader (for example, OW-ADR-0002
  revisited at `saphyr` 1.0). That Warrant must declare
  `docs/GRAMMAR.md` and the plant, or they report the drift.

---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f26-7ba0-a64b-2618b983ba43
role: intent
jurisdiction: authored
order: 10
classification: internal
---


# Intent

## Problem

The SAS is one 5,840-line Markdown file, governed as a whole. A revision
record (`docs/sas/revisions/<version>.toml`) pins the file's sha256 and a
snapshot of §106. Nothing smaller than the document has an identity or a
digest.

- **Section references are unchecked prose.** Sixteen amendments cite a
  section as `sas://WAR-SAS-43.5`, `-46.3`, `-59`, `-96.2` or `-101` in
  `governing_adr_or_policy`. `traceability.rs` parses only
  `<NS>-SAS-RQ-NNN`, and §105 lists only `sas://<requirement-id>`. A
  section that does not exist would pass.
- **Currency is all or nothing.** A Warrant authorized under
  `0.1.0-draft.1` rests on the whole document at that digest. Nothing can
  say whether the sections it relies on changed since. A throwaway split
  of the two revisions' bytes (2026-09-23) found 10 of 125 top-level
  units changed between `0.1.0-draft.1` and `1.1.0`, and none of the
  sections the sixteen amendments cite.
- **`war sas diff` sees only §106.** A revision that rewrites §62's
  meaning without touching §106 shows no change there.
- OW-WAR-0113 set SAS atomization aside as a non-goal ("the prose stays one
  file under §101"). OW-WAR-0114 placed it at
  `roadmap://OW-PHASE-3/sas-atoms`.

## Desired Outcome

This Warrant is the first of two steps. It gives every SAS section an
identity, a digest and a currency, derived from the pinned document. It
does not move where the SAS is written.

- **Sections, split losslessly.** A section is the preamble, a `# Part` or
  `# Appendix` heading, or a numbered `## N.` section with its `### N.M`
  subsections, found outside code fences. Joining the sections in order
  gives back the document byte for byte.
- **A generated index.** `war compile` writes
  `docs/sas/generated/SECTIONS.json` and `SECTIONS.md`: each section's id,
  title, byte range and sha256, and the revision whose bytes they come
  from. `war check --generated` refuses a hand edit.
- **Section references resolve.** A `sas://<NS>-SAS-<n>[.<m>]` reference in
  an amendment must name a section or subsection of the SAS revision the
  Warrant is pinned to. One that does not is an error.
- **A section has its own currency.** For a cited section, `war check`
  compares its digest at the Warrant's pinned revision with the latest
  accepted revision: unchanged passes, changed warns, and bytes that
  history no longer holds are UNKNOWN.
- **`war sas diff` names changed sections** beside its §106 comparison.
- **The decision is recorded.** OW-ADR-0028 (proposed) states the section
  identity, the two steps, and the SAS text a later revision would carry
  (the `sas://<NS>-SAS-<section>` form in §105).

## Non-goals

- Step two: making sections the authored atoms and the document their
  generated projection. That moves the SAS's source and its §101
  governance, and waits for the owner to accept OW-ADR-0028. A later
  Warrant carries it.
- Any change to the SAS text. The §105 addition is proposed in the ADR;
  the SAS revision that carries it is the owner's separate act.
- A section record in the revision schema. `oh.war/sas-revision/v1` and
  the schema pack do not move; section digests are derived from the bytes
  the record already pins.
- Section references outside amendments (atoms' prose, ADRs). Only the
  structured field is checked.
- The Knowledge Fabric or Liminal controlled document of §101.1.

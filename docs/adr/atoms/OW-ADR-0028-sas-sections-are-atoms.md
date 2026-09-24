---
schema: oh.war/atom/v1
adr_uuid: 01a0d3cd-27e1-7e4c-a236-7a3bbdb86f88
local_alias: OW-ADR-0028
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0d04c-5f26-7ba0-a64b-2618b983ba43"
---

# ADR OW-0028: SAS sections are atoms — identity now, authored source later

## Status

Proposed by the performer under OW-WAR-0125. Authorizing that Warrant adopts
step one below, as OW-ADR-0022 is adopted with OW-WAR-0113. Nothing here
changes the SAS: the §105 wording at the end is a **proposal** for a
separate SAS revision, which is the owner's act under §101 (U-001).

## Context

The SAS is one Markdown file, governed as a whole. A revision record
(`docs/sas/revisions/<version>.toml`) pins the file's sha256 and a snapshot
of §106. Nothing smaller than the document has an identity or a digest, so:

- sixteen amendments cite a section in `governing_adr_or_policy`
  (`sas://WAR-SAS-43.5`, `-43`, `-46.3`, `-59`, `-96.2`, `-101`), and by
  2026-09-24 twenty-seven more cite `sas://WAR-SAS-44.8`. §105 lists only
  `sas://<requirement-id>`, and `traceability.rs` reads only requirement
  references. A citation of a section that does not exist passed;
- a Warrant pinned to `0.1.0-draft.1` rests on the whole document at that
  digest, and nothing can say whether the sections it relies on changed
  since. Between `0.1.0-draft.1` and `1.1.0`, 10 of 124 units changed, and
  none of the cited sections did;
- `war sas diff` compared §106 only, so a revision that rewrote §62 without
  touching §106 showed no change.

OW-WAR-0113 set SAS atomization aside ("the prose stays one file under
§101"); OW-WAR-0114 placed it at `roadmap://OW-PHASE-3/sas-atoms`.

## Decision

1. **A section is a unit of the pinned document, found outside code
   fences.** The units are the preamble (everything before the first
   boundary), each `# Part <n>` and `# Appendix <x>` heading with the prose
   up to the next boundary, and each numbered `## <n>.` section with its
   `### <n>.<m>` subsections. A boundary is one of those heading forms at
   column zero outside a fenced block; `###` and deeper stay inside their
   section. A heading inside a fence is an example, not a section. Joined
   in order, the units are the document byte for byte, so a revision's
   sha256 remains the digest of the join and a section's sha256 is a digest
   of part of it.
2. **A section's identity is its number as written in the accepted
   revision:** `<NS>-SAS-<n>` for a numbered section, `<NS>-SAS-<n>.<m>`
   for a subsection, where `<NS>` is the namespace the revision's §106 rows
   carry (`WAR`). It is cited as `sas://<NS>-SAS-<n>` or
   `sas://<NS>-SAS-<n>.<m>`. Once a revision carrying a number is accepted,
   that number is **never renumbered or reused** for other content, for
   the reason §106 ids are append-only: every citation of it would silently
   change meaning. A section may be added, amended or withdrawn (its
   heading kept, its prose saying so); it may not move. The structural
   units (`preamble`, `part-<n>`, `appendix-<x>`) have digests and no
   citation form.
3. **Step one — sections derived from the pinned document (OW-WAR-0125).**
   The document stays the authored source under §101. `war compile`
   derives `docs/sas/generated/SECTIONS.json` and `SECTIONS.md` from the
   document in force, naming its sha256 and revision, drift-checked like
   `NORMATIVE.*`. `war check` resolves each section citation in an
   amendment against the revision its Warrant is pinned to
   (`sas.section-ref`, an error when it names nothing) and compares the
   cited section's digest there with the latest accepted revision
   (`sas.section-current`: pass, warning when changed, UNKNOWN when history
   no longer holds the bytes). An older revision's bytes are read from
   Git by the digest its record pins; nothing is fetched. `war sas diff`
   names the sections a candidate adds, removes or changes.
4. **Step two — sections become the authored source (a later Warrant).**
   Each section becomes an atom, and the document becomes their generated
   projection, compiled and drift-checked as a Warrant's parent is (Law 1).
   That moves where the SAS is written and how §101 governs it, so it waits
   for the owner to accept this ADR, and is carried by its own Warrant.
   The rule against renumbering (2) is enforced there, where a section has
   an authored identity to hold; step one reports a broken citation and
   does not yet refuse a renumbering revision.

## Proposed SAS text (a proposal, not a change)

**This is proposed wording for a separate SAS revision. It is not part of
the SAS until the owner proposes and accepts a revision carrying it
(§101.2, §101.3). OW-WAR-0125 changes no SAS byte.**

In §105, after `sas://<requirement-id>`:

```text
sas://<NS>-SAS-<section>
```

and, after the list:

> A section reference names a numbered section (`<n>`) or subsection
> (`<n>.<m>`) of the SAS revision the citing record is pinned to, by the
> number that revision gives it. Section numbers are stable: an accepted
> revision SHALL NOT renumber a section or reuse a number for other
> content.

## Why not the alternatives

- **Keep citing requirements only.** The corpus already cites sections,
  forty-three times, because the reason for an amendment is often a
  paragraph (§44.8's binding rule), not a row of §106. Refusing the form
  would push those citations back into unchecked prose.
- **Make sections the authored source now.** It moves the SAS's source and
  its §101 governance in the same change that first gives sections an
  identity; the identity should be shown to hold on the five recorded
  revisions before anything is built on it.
- **Record section digests in the revision record.** It would change
  `oh.war/sas-revision/v1` and every record under it, for facts that are
  already derivable from the bytes the record pins.

## Consequences

- Section citations are checked, and a citation of nothing is an error.
- A Warrant can learn that a section it relied on changed, without the
  all-or-nothing `sas.pin-superseded` warning being the only signal.
- In a shallow clone, every Warrant pinned to an older revision reports
  `sas.section-current` UNKNOWN (R-002): the bytes are not there, and a
  pass would be a guess. A checkout that runs `war check` over this corpus
  needs the SAS's history.
- A revision that renumbers a section breaks every citation of it; step one
  reports each one (R-001), step two refuses the revision.

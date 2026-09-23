---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f26-7ba0-a64b-2618b983ba43
role: assurance
jurisdiction: authored
order: 60
classification: internal
---


# Assurance

## Acceptance Obligations

### OBL-001 — the split is lossless and respects fences
- **scope:** the five recorded SAS revisions' bytes in this repository's
  history, and one planted fence. No claim about Markdown this SAS does
  not use (setext headings, indented code blocks).
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - for each of the five revisions, `join(split(bytes))` has the sha256
    the revision record pins;
  - a `## 999. Fake` line inside a fenced block produces no section `999`,
    and the plant fails if it does.

### OBL-002 — the section index is generated and a hand edit is refused
- **scope:** `docs/sas/generated/SECTIONS.json` and `SECTIONS.md`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - after `war compile`, both files exist and name the document's sha256
    and revision `1.1.0`;
  - changing one digest in `SECTIONS.json` makes `war check --generated`
    fail with a drift finding naming the file.

### OBL-003 — a section reference that names nothing is refused
- **scope:** `governing_adr_or_policy` in amendments, in this repository.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - `sas://WAR-SAS-999` and `sas://WAR-SAS-43.9`, each planted in a copy of
    an amendment, fail `war check` with `sas.section-ref`;
  - the sixteen existing references produce no `sas.section-ref` error.

### OBL-004 — currency is per section, and unknown when history cannot say
- **scope:** Warrants whose amendments cite a section, pinned to
  `0.1.0-draft.1`, compared with `1.1.0`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the sixteen existing references each report `sas.section-current`
    as a pass (the throwaway split found none of them changed);
  - a plant that cites a section among those that changed between the two
    revisions reports a warning naming both revisions;
  - in a shallow clone, the same reference is UNKNOWN, not pass and not
    warning.

### OBL-005 — war sas diff names exactly the sections that changed
- **scope:** `war sas diff` against candidates built by the plant.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the document against itself: no section named;
  - one word changed in §62.3: section 62 named and no other;
  - a §106 row removed: the existing refusal still fires, unchanged.

### OBL-006 — the decision is recorded as a proposal, and no SAS text changes
- **scope:** `OW-ADR-0028` and the SAS document.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - the ADR's status is `proposed`, and it names this Warrant in
    `governs`;
  - its proposed §105 text is marked as a proposal for a separate SAS
    revision;
  - the SAS document's sha256 after delivery equals revision `1.1.0`'s.

## Gate Adequacy

Required at `basic`. The load-bearing plant is OBL-001's fence case: a
splitter that treats a heading inside an example as a section would give
the SAS's own examples an identity, and every later check would stand on
it. OBL-003's refusal is what makes the sixteen passing references mean
anything.

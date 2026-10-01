---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ee5-7ba2-9dc3-b9c50dcc6ba1
role: intent
jurisdiction: authored
order: 10
classification: internal
---


# Intent

## Problem

The accepted SAS says little about the grammar of a Warrant's source files,
and `war` implements more than the SAS says. No document holds both, so
nobody can say whether another tool that reads these files conforms.

- §62 says "Markdown with YAML frontmatter" and gives one example. §62.1
  lets structured atoms use YAML or canonical JSON. §62.3 says "unknown
  required fields fail; namespaced optional fields are preserved".
- `war` reads a restricted subset instead of YAML (OW-ADR-0002 for
  frontmatter, OW-ADR-0003 for `.yaml` atoms). The subset refuses nested
  mappings. §62's own example has a nested mapping (`holder:` then
  `kind: git`), so this tool refuses the SAS's example.
- The Markdown body has grammar the tool depends on and the SAS never
  states: section headings (`sections.rs`), the obligation form
  `### OBL-NNN — statement` with `- **scope:**`, `- **gate:**` and
  `- **evidence:**` bullets (`obligation.rs`, `gate.rs`). `obligation.rs`
  says this form was "fitted to the corpus".
- The atom header is parsed and not checked. `repo.rs` reads only
  `jurisdiction` from it. Nothing compares `schema`, `warrant_uuid`,
  `role` or `order` with the manifest. An atom copied from another
  Warrant, or one whose header names another role, passes `war check`.
- The product spec lists "minimum document schema, precise Markdown/header
  grammar … and assignment of individual requirements to standard
  conformance versus reference-tool workflows" among the engineering
  contracts still to specify.

## Desired Outcome

- **One grammar document**, `docs/GRAMMAR.md`, for the atom format in force
  (SAS 1.1.0). For each construct it states:
  - the SAS clause that governs it, quoted, or "none";
  - **standard conformance**: what any conforming tool must accept and
    must refuse, derived only from the SAS text;
  - **tool conformance**: what `war` accepts and refuses, with the
    diagnostic rule it reports;
  - every place the two differ, named as a divergence.
- **The header is checked.** A new rule, `atom.header`, compares each
  Markdown atom's `schema`, `warrant_uuid`, `role` and `order` with the
  manifest entry that declares it. A missing or different value is an
  error. ADR atoms keep their own check (`adr.malformed`).
- **Plants for the grammar.** Each accepted and refused construct the
  document lists is exercised by a plant, so the document cannot drift
  from what the tool does without the battery failing.
- **SAS changes are proposed, not made.** Where the SAS and the tool
  disagree (the §62 example first), `docs/GRAMMAR.md` proposes the text a
  SAS revision would carry. Accepting it is the owner's separate act.

## Non-goals

- The RC.3 human-first document format (footer TOML, units). It has its
  own draft contract (`docs/sas/drafts/1.0.0-rc.3/format-contract.md`)
  and its own Warrant (OW-WAR-0075). This Warrant does not decide which
  format is the standard.
- Changing what the frontmatter or structured readers accept. OW-ADR-0002
  and OW-ADR-0003 stand; a divergence is recorded, not fixed here.
- Refusing unknown non-namespaced header keys or unknown manifest keys.
  Whether §62.3 requires it is an open question (Basis, U-002); the
  document records the tool's current behaviour.
- A SAS revision. The proposed text is a deliverable; the revision is a
  separate act by the owner.
- Migrating any existing atom. The corpus already passes the new rule
  (Basis, A-001).

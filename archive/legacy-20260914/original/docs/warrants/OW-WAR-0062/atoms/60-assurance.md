---
schema: oh.war/atom/v1
warrant_uuid: 01a064fc-a6f2-7c43-bed9-fa248b771712
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the rule is in the controlled document and cannot leave it quietly
- **scope:** SAS §6.10 at revision 0.1.0-draft.3.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `docs/sas/revisions/0.1.0-draft.3.toml` accepted, its digest
  matching the document (`sas.pinned`); the core test
  `section_6_10_states_that_a_sas_and_a_warrant_are_the_same_class_of_artifact`
  passing, and failing when the sentence is removed.

### OBL-002 — every level has a definition a reader can act on
- **scope:** the nine levels of §6 plus Release, Objective and Requirement.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** §6.10's table names, for each, the object, what it is, who
  writes it, what governs it and what reads it; `docs/DEFINITIONS.md`
  carries one paragraph per object and the two decisions.

### OBL-003 — the next author sees the rule before writing
- **scope:** `war new`, the README.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a manifest created by `war new` begins with the three
  pointer lines; the README's status section points to the computed ladder
  and to the definitions and asserts no count.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** can the rule be lost, contradicted, or unseen? The
attacks: the sentence edited out of §6.10; a restatement drifting from the
SAS; a new author never reaching the page.

**Executed attacks:** the core test, run against the real document — removing
the sentence fails it; the SAS pin — any byte change without a new accepted
revision is `sas.digest-drift`; the manifest template — every `war new`
carries the pointer, checked by the test that creates a draft.

- **outcome:** no_counterexample

## Residual Risk

- "Same class of artifact" is stated as governance and structure, not made
  executable: the SAS does not compile through `war`. A reader can still
  hold the two apart in their head; the text is what corrects them.
- The restatement in `docs/DEFINITIONS.md` can drift from §6.10. The SAS
  wins by rule; nothing checks the two agree.

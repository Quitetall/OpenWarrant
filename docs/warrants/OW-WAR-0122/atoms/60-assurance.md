---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ee5-7ba2-9dc3-b9c50dcc6ba1
role: assurance
jurisdiction: authored
order: 60
classification: internal
---


# Assurance

## Acceptance Obligations

### OBL-001 — an atom header that disagrees with its manifest is refused
- **scope:** non-ADR `.md` atoms loaded by `repo.rs`, exercised by the
  four header plants in `57-grammar.sh`. No claim about `.yaml` atoms or
  ADR atoms.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** each plant (`role` changed, `warrant_uuid` changed,
  `order` changed, `schema` removed) makes `war check` exit non-zero with
  `atom.header`, and the message names the key and both values (or the
  missing key).

### OBL-002 — the rule refuses nothing in the current corpus
- **scope:** this repository's corpus at the commit the plant runs on.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war check` on the unmodified corpus reports no
  `atom.header` error; the battery's clean run is the record.

### OBL-003 — every construct the grammar document lists is observed doing what the document says
- **scope:** the constructs in `docs/GRAMMAR.md`'s tables whose plant
  column is filled. A construct marked "unspecified" makes no claim.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the refused constructs (anchor, tag, flow collection, block scalar,
    duplicate key) each fail with `atom.frontmatter` naming the construct;
  - §62's example header, verbatim, fails with `atom.frontmatter` naming
    the nested mapping, matching the divergence the document records;
  - the positive controls pass: §62's example without `holder`, and a
    namespaced key that survives into the compiled IR;
  - the drift control fails if the document cites a rule id no plant
    produced.

### OBL-004 — the document separates the standard from the tool, and proposes rather than makes SAS changes
- **scope:** `docs/GRAMMAR.md`.
- **gate:** `gate://document.review@1.0.0`
- **evidence:** every table row has both a standard column and a `war`
  column; every "standard" cell quotes a SAS clause or says
  "unspecified"; every divergence has a proposed SAS text marked as a
  proposal; no row claims a behaviour without a plant or a cited source
  line.

## Gate Adequacy

Required at `basic`. The load-bearing plant is OBL-001's: an atom whose
header names another Warrant or role is the silent substitution this
Warrant exists to stop, and OBL-002 shows the rule does not buy that by
breaking the corpus.

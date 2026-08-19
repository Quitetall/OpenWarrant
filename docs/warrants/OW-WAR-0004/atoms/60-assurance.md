---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-75b4-9586-0aae240f38bc
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — both projections render from one Basis

- **scope:** the five Warrants in `docs/warrants/`, `full_warrant` and
  `canonical_json` views only. No claim about the other seven views in §17.5.
- **evidence:** generated files exist, each carrying a §17.1 header naming its
  WAR, basis digest, contract revision, and source manifest.

### OBL-002 — recompilation on an unchanged tree changes nothing

- **scope:** two consecutive `war compile` runs over an unmodified working tree.
- **evidence:** `git diff --exit-code` over the generated tree exits 0.
- **why it is a separate obligation:** this is the property that decides whether
  the drift check survives contact with daily use. A compiler that embeds a
  timestamp passes OBL-001 perfectly and makes OBL-003 fire on every run, and
  the response to a check that always fires is to stop running it.

### OBL-003 — planted drift is REFUSED

- **scope:** a single-byte mutation of a committed generated parent.
- **evidence:** `war check --generated` exits non-zero and names the file.
- **note:** OBL-001 and OBL-002 are both satisfiable by a drift check that
  always returns "no drift". Only this obligation distinguishes the two.

## Gate Adequacy

Not required at `basic` (§25.1). Asked anyway:

> Could a reader be misled by a generated document that passes all three?

Yes, in one way. The renderer omits inapplicable optional roles per §16.1, so a
Warrant with no Execution section and a Warrant whose Execution section the
renderer failed to emit produce identical output. Nothing here distinguishes
them. The mitigation is partial — the §17.1 header names the source manifest, so
a suspicious reader can check — and it depends on the reader being suspicious,
which is not a control. Recorded as a real gap rather than argued away.

## Residual Risk

- Seven of the nine §17.5 views are unimplemented. A caller requesting one gets
  an explicit unimplemented error, never an empty document that reads as an
  answer.
- No source maps, so a parent edit is refused rather than mapped. This is
  permitted by §17.4 and is the safe direction, but it means an author who edits
  the wrong file is told "no" without being told where to go instead.

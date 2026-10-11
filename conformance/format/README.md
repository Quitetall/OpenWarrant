# Format 1.0 conformance corpus

This directory is the corpus of section 11.1 of
[`docs/format/FORMAT-1.0.md`](../../docs/format/FORMAT-1.0.md). Each case was
derived from a sentence of the specification. None was derived from a tool's
output. Another implementation of the format can run these cases to show
that it reads documents the way the specification says.

Licence: the corpus, `reader.py` and `run.py` are Apache-2.0, the same as
the schemas. The specification's text is CC-BY-4.0 (`docs/format/LICENSE`).

## Layout

```text
conformance/format/<NNN>-<slug>/
  case.toml            # id, kind (accept | refuse | unknown), section, rule, says
  input/**             # a repository tree: the documents under test
  args                 # context cases only: the arguments of `war context`
  expected/model.json  # accept: the normalized model (below), or
  expected/refusal     # refuse: one line, `<rule id> <path>:<line>`, or
  expected/unknown     # unknown: one line, `<rule id>`
```

- **Numbering:**
  - `0NN` cases are accepted;
  - `1NN` are refused;
  - `2NN` read `UNKNOWN`;
  - `3NN` are context cases.
- **`says`** quotes the sentence the case tests.
- **`rule`** names the section 11.2 rule id the case expects.
- **Coverage:** every rule id of section 11.2 is expected by at least one
  case. `run.py` fails if one is not.

## What is compared

- **Refuse cases:** the set of refusals the reader reports must be exactly
  the one line in `expected/refusal`. A tool that also reports a consequence
  of that fault fails the case. For example, a refused obligation must not
  also be reported as a missing section.
- **Unknown cases:** the set of `UNKNOWN` rule ids must be exactly the one in
  `expected/unknown`, with no refusal.
- **Accept cases:** the reader must report no refusal and no `UNKNOWN`, and
  its normalized output must equal `expected/model.json` after RFC 8785
  canonicalization. The normalized output is:

  ```text
  { records:   [{id, type, title, fields, ext, content_digest}]   sorted by id
    relations: [{from, kind, to, to_revision?}]                    sorted
    warnings:  ["<rule id> <path>:<line>"]                         sorted }
  ```

  - **Records** are only those whose source is a file of the case's
    `input/`.
  - **`content_digest`** (section 9) covers each record's title, fields,
    relations and normalized body.
  - **`revision`, `line` and `span` are left out.** They are exact-bytes
    values whose rules section 5 already fixes.
  - **Containment relations are dropped.** A reader that also emits a
    `part_of <document>` relation for every record of a work document says
    nothing that `document` does not, so the comparison drops it.
- **Context cases (`3NN`):** these are refusals of the context compiler
  (section 10.3). A reader that is not a context compiler skips them.

## Running it

```sh
cargo build -p openwarrant-cli
python3 conformance/format/run.py --reader war --known conformance/format/known-war.txt
python3 conformance/format/run.py --reader python
python3 conformance/format/run.py --reader war -v 044      # one case, verbosely
```

- **`--reader war`** copies each case's `input/` into a scratch program
  (made by `war init` with a namespace no case uses). It then reads
  `war model --json` and `war --json check`.
- **`--reader python`** runs `reader.py`. That is a second, independent
  reader, written from the specification alone in the Python standard
  library. It shares no code with `war`.
- **The gate:** plant `conformance/plants.d/183-format-corpus.sh` runs the
  war reader. It also proves that the runner fails when an expected file is
  changed.

A new implementation passes when `run.py` reports no failure. To run your
own reader, adapt `War.read` in `run.py`: it needs a function from a case's
`input/` tree to the normalized output above.

## Known divergences

`known-war.txt` lists the cases `war` fails today, each with its reason.
- `run.py --known` reports those cases as `known`. A listed case that
  starts passing fails the run.
- A case leaves the list when `war` changes, or when the specification and
  the case are amended together. It never leaves because someone edited an
  expected file to match `war`.

## Readings

Where the specification leaves a choice open, the corpus takes one reading,
and `reader.py` marks it `READING:`. These are proposed for an amendment of
FORMAT-1.0.md and are listed in the report that introduced this corpus:

- **The document record:**
  - Its fields leave out `format`, so that section 9's same digest in every
    encoding can hold.
  - Its body is the preamble without its `# ` title line.
- **A section's body** skips one blank line after its heading, as a record
  without an attribute block does.
- **An empty or over-long record title** is `format.title-missing`.
- **Line numbers:**
  - A missing closing frontmatter fence is reported at line 1.
  - A missing required frontmatter key is `format.field-missing` at line 1.
  - A missing section is reported at line 1.
  - A duplicate document is reported at the path that sorts second.
- **Relation lines and targets:**
  - A line whose first word is a core or dotted relation kind is a relation
    line, so a target that does not parse is `format.relation-target`.
  - A bare item id is the document's own item, even when no item has it.
- **Items:**
  - An item's title is its text, without the id group and the `done` suffix.
  - A box line whose id group is not at the end has no id group.
- **Records documents** (non-work documents) live at
  `docs/records/<area>/*.md`.

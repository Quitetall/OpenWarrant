# Format 1.0 conformance corpus

This directory is the corpus of section 13.1 of
[`docs/format/FORMAT-1.0.md`](../../docs/format/FORMAT-1.0.md). Each case was
derived from a sentence of the specification. None was derived from a tool's
output. Another implementation of the format can run these cases to show
that it reads documents the way the specification says.

The corpus follows draft 3 (`1.0.0-draft.3`). Questions the draft leaves
open are in [`docs/format/draft-3-questions.md`](../../docs/format/draft-3-questions.md).

Licence: the corpus, `reader.py`, `context.py`, `tracker.py` and `run.py`
are Apache-2.0, the same as the schemas. The specification's text is
CC-BY-4.0 (`docs/format/LICENSE`).

## Layout

```text
conformance/format/<NNN>-<slug>/
  case.toml             # id, kind (accept | refuse | unknown), section, rule, says
  input/**              # a repository tree: documents and profiles/
  args                  # context, tracking and apply cases: the arguments, one line
  at                    # tracking and apply cases: the time, RFC 3339
  journal/<alias>.jsonl # tracking: the work document's journal
  journal/<alias>.committed.jsonl   # tracking: the journal as committed
  claims/*.lock         # tracking: the claim store
  acts/*.json           # tracking: acceptance acts taken as verified
  package.md            # apply: the package given back
  expected/model.json   # accept: the normalized model (below), or
  expected/context.md   # accept, context: the exact output, or
  expected/states.json  # accept, tracking: the states, or
  expected/next.json    # accept, tracking: the candidates of `next`, or
  expected/after/**     # accept, tracking and apply: files after the operation, or
  expected/refusal      # refuse: one line, `<rule id> <path>:<line>` (`<rule id>` for context), or
  expected/unknown      # unknown: one line, `<rule id>`
```

- **Kind of case:**
  - a case with an `at` file is a tracking case;
  - with `at` and a `package.md`, it is an apply case;
  - with `args` and no `at`, it is a context case;
  - any other case is a model case.
- **Numbering:**
  - `0NN` cases are accepted;
  - `1NN` are refused;
  - `2NN` read `UNKNOWN`;
  - `3NN` are context cases;
  - `4NN` are tracking cases (states, `next`, claims, journals, checks);
  - `5NN` are apply cases.
  - A tracking case that reads `UNKNOWN` is a `2NN` case.
- **`says`** quotes the sentence the case tests.
- **`rule`** names the section 13.2 rule id the case expects.
- **Coverage:** every rule id of section 13.2 is expected by at least one
  case. `run.py` fails if one is not. The exception is
  `context.span-unknown`, which 13.2 lists but no section defines
  (draft-3-questions.md Q1).
- **Pairs:** each rule new in draft 3 has a case that is accepted and one
  that is refused, `UNKNOWN` or warned. Every claim has a case that is
  refused.

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
- **Context cases (`3NN`):**
  - `args` is a line of `war context` arguments: a target and `--budget`,
    `--depth`, `--preset` and `--at`. `--index` asks for the index of 10.2.
  - An accept case compares the output byte for byte with
    `expected/context.md`.
  - A refusal is one line, `<rule id>`.
- **Tracking cases (`4NN`, and `2NN` with an `at`):**
  - `args` names one operation:
    - `states`;
    - `next`;
    - `claim <target> --actor A [--steal]`;
    - `done <item> --actor A --base <revision>`;
    - `note <target> --actor A --note TEXT`;
    - `evaluate --actor A`, which ticks what the recorded check runs pass
      (12).
  - `input/` becomes a git working tree and is committed.
  - `journal/<alias>.committed.jsonl`, when present, is committed at the
    document's evidence directory (`docs/<place>/<alias>/journal.jsonl`)
    first. `journal/<alias>.jsonl` is then written there uncommitted.
  - `claims/` is the claim store, `<git common dir>/openwarrant/claims/`.
    Version control carries no modification time, so each claim file's
    modification time is its `since_unix`.
  - `acts/*.json` are acceptance acts (Appendix C), each `{act: "accept",
    records: [[id, revision, content_digest]…]}`. They are taken as
    verified: the corpus carries no signature.
  - `states.json` is `{states: [{record, kind, value, refines?, lapsed?}],
    warnings}`. The states are sorted by `(record, kind, value)`.
    `provenance` and `facet` are left out (draft-3-questions.md Q20).
  - `next.json` is `{candidates: [<global id>…], warnings}`, in order.
  - A tracking refusal is `<rule id> <path>:<line>`:
    - for a journal, its repository path and the faulty line (the
      committed line that is gone, for `work.journal-rewritten`; the
      existing event's line, for `work.idempotency-conflict`);
    - for a claim, `claims/<file>:1`;
    - for a stale write, the document and the item's line.
- **Apply cases (`5NN`):**
  - `args` is `apply --actor A`.
  - The package is `package.md`, made by the `human` preset (10.4) and
    then edited.
  - `expected/after/**` holds the documents after `apply`. Journals are
    compared only where `after/` lists them: an appended event carries a
    fresh id and time.
  - A refusal names the document and the marked line, or, for a block that
    does not parse, the line of the fault.

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
- **`--reader python`** runs the second, independent implementation,
  written from the specification alone in the Python standard library. It
  shares no code with `war`:
  - `reader.py` for model cases;
  - `context.py` (10.2 to 10.5) for context cases;
  - `tracker.py` (10.5, 11, 12) for tracking and apply cases.
- **Built-in profiles:** they are loaded from `schemas/format/1.0/profiles/`
  (7.6: the files are normative). A repository's own are `profiles/*.toml`.
- **`--reader war`** has no adapter for tracking and apply cases yet. Those
  cases are in `known-war.txt`.
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
- **Draft 3:** draft-3-questions.md asks each of these. In brief:
  - Profile data may declare keys and sections (Q2).
  - A repository file that replaces a built-in profile names it in
    `extends` (Q3).
  - A type for which no field is declared accepts any field (Q5).
  - 4.5's requirement fields are the `spec` profile's (Q6).
  - Undotted record types are not closed (Q7).
  - The field shapes read as Q4 gives.
  - `context.statement-shape` checks bytes only and is reported at the
    heading (Q14).
  - `context.section-ref` applies to specification documents (Q15).
  - A reverse domain name has three or more labels (Q17).
  - The walk follows containment `part_of` (Q21).
  - The document record's span is its frontmatter and preamble (Q22).
  - Every packet header names its preset (Q23).
  - A preset with its own steps names its walk after itself, and `review`
    walks `in:evaluates`, `out:evaluates` (Q24).
  - The rendering of `resume` follows Q25.
  - The journal digest covers every journal (Q26).
  - Event `target` is a global id (Q33).
  - A box is `done` only with a `work.item_done` (Q18).
  - The verdicts are `PASS`, `FAIL` and `UNKNOWN` (Q42).

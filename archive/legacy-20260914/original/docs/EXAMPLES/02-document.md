# Example 2 — a document Warrant: a research memo with a gate

OW-WAR-0065, *Research memo: which tokenizer approximation OpenWarrant uses
for §33.7 budgets*. Profile `delivery`, assurance `basic`. Its one
deliverable is a Markdown file, and its gate is the document work kind's:
`gate://document.review@1.0.0`.

## What "done" means for a document

`war document review` checks four things a gate can check, and nothing it
cannot:

| rule | what it refuses |
|---|---|
| `document.undigested` | the memo is missing, or its bytes moved since the deliverable record took its digest |
| `document.citation-missing` | a relative citation (`[text](path)`) names a file that does not exist; URLs are recorded, never fetched |
| `document.placeholder` | `TODO`, `TBD`, `FIXME`, `lorem ipsum`, `<placeholder>` in prose (fenced code is exempt) |
| `document.review-absent` | an obligation with no `established` verification from someone other than the performer |

Whether the memo's argument is any good is the reviewer's job, and OBL-002
says so: "the decision follows from the numbers" is established by a person
or a separate model reading the memo, not by the gate.

## The records

```bash
war show OW-WAR-0065
cat docs/research/tokenizer-approximation.md
cat docs/warrants/OW-WAR-0065/deliverables.toml     # content_digest of the memo
war document review OW-WAR-0065                     # today: NOT READY — review-absent
```

The Warrant was created with `war new`, its atoms written by hand: an intent
that names what is out of scope (calling a tokenizer), a basis that records
where the numbers came from, a work order with one deliverable, one milestone
with one agent stage whose `context_sections` narrows its Dispatch to the
Deliverables section, and two obligations bound to the document gate.

The memo's numbers are `wc` counts of files in this repository, with the
command beside them; the gate cannot check that they are true, so OBL-001
asks a reviewer to re-run one.

## The loop from here

```bash
war authorize OW-WAR-0065 && war sign OW-WAR-0065 --ssh-sign     # human
war evidence record OW-WAR-0065          # runs document.review; today it records NOT READY, honestly
war verify OW-WAR-0065 --performer claude --bundle               # the memo, its atoms, its record, in one file
# hand the bundle to a separate context; ingest the verdicts
war verify OW-WAR-0065 --response verdicts.toml
war evidence record OW-WAR-0065          # now the gate passes: review present
war resolve OW-WAR-0065 && war sign OW-WAR-0065 --ssh-sign       # human
```

## What to copy

- one Markdown deliverable, content-addressed, with its digest in
  `deliverables.toml`;
- citations by path, so the gate can hold them;
- an obligation whose evidence is a *reviewer's* verdict, not the author's
  claim — the gate refuses closure until it exists.

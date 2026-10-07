# Documents from records

Write each fact once, as a record. Get a PRD, an architecture view, a test
plan and an agent's brief from the same records, kept in step for you.

A **record** is one statement with an id: an outcome, a requirement, a
constraint, a decision. A **document** is a view of records. You never edit
a document; you edit the records, and `war compile` rewrites every document
that shows them. `docs/TYPES.md` is the reference. This page walks through
it.

## A PRD in three commands

**1. Write the records.** One Markdown file in an area directory, here
`docs/records/csv-export/10-records.md`:

```markdown
---
schema: oh.war/records/v1
profile: delivery
---
# CSV export

## OUT-cx1 · outcome

An analyst can take a report's rows into a spreadsheet without retyping them.

## REQ-cx1 · requirement
implements OUT-cx1

Every report has an "Export CSV" action that downloads its visible rows.

## CON-cx1 · constraint
constrains REQ-cx1

An export never includes a column the viewer cannot see on screen.
```

Each `## <ID> · <type>` line opens a record. A line such as
`implements OUT-cx1` inside one is a relation. The `profile` names who
decides which types and relations are allowed. In this repository,
`profiles/delivery.toml` allows these.

Add what a PRD also needs, non-goals and open questions, in a second file.
The `prd` type declares those nouns, so that file is governed by `prd`:

```markdown
---
schema: oh.war/records/v1
profile: prd
---
## NG-cx1 · non_goal

Scheduled or emailed exports.

## Q-cx1 · question

Should an export of more than 100,000 rows run in the background?
```

**2. Check them.**

```text
war check
```

It names each refusal: a duplicate id, a type the profile does not declare,
a requirement that serves no outcome. When nothing is refused, it says
`records.well-formed`.

**3. Read the PRD.**

```text
war render prd --of csv-export
```

The PRD prints to your terminal and nothing is written:

```markdown
# csv-export: product requirements

## Outcomes

> An analyst can take a report's rows into a spreadsheet without retyping them.
>
> — OUT-cx1

## Requirements

- **REQ-cx1.** Every report has an "Export CSV" action that downloads its visible rows. *Implements OUT-cx1; constrained by CON-cx1.*

## Constraints

- **CON-cx1.** An export never includes a column the viewer cannot see on screen. *Constrains REQ-cx1.*
...
## Sources

| Record | Type | Revision | Source |
|---|---|---|---|
| OUT-cx1 | outcome | `3f9c…` | docs/records/csv-export/10-records.md:7 |
...
```

## Keep it

To keep the PRD with the code, declare it beside the records, in
`docs/records/csv-export/documents.toml`:

```toml
schema = "oh.war/documents/v1"
title = "CSV export"

[[document]]
type = "prd"

[[document]]
type = "architecture"

[[document]]
type = "test-plan"
```

Then run `war compile`. It writes `docs/records/csv-export/generated/prd.md`
and the other declared documents. Commit them with the records.

- **Change a record, recompile.** Each document that shows the record
  changes. A document that does not show it stays byte for byte the same.
  `war impact REQ-cx1` lists those documents before you make the change.
- **Never edit a generated file.** `war check --generated` renders every
  declared document again and reports `projection.drift` for any file that
  differs. That covers a hand-edit, and also a file under `generated/` that
  no declared document produces.

## The four types

| type | for | shows |
|---|---|---|
| `prd` | product people | outcomes, requirements and what they serve, constraints, non-goals, open questions |
| `architecture` | engineers | decisions with the alternatives they were chosen over and their consequences, interfaces, constraints; it names requirements by id but does not restate them |
| `test-plan` | whoever checks the work | each requirement and constraint with the Warrant obligations that `evaluates` it, the evidence each needs, and its verdict. A requirement that nothing evaluates says it is untested. A verdict given against an earlier revision is marked stale. |
| `agent-packet` | an agent doing one stage | the records that stage needs, from one root record: what to build, why, the rules, the rejected alternatives, how it will be checked. Each record shows its source and revision. The packet has a byte budget. |

An agent packet is bounded on purpose. Declare one per stage, with the
stage's root record:

```toml
[[document]]
type = "agent-packet"
title = "the export action (REQ-cx1)"
roots = ["REQ-cx1"]
```

When a packet goes over its budget (16000 bytes by default), war refuses it
by name and lists the records that cost the most. It never writes a cut-down
packet: an agent that read one would not know what it was missing. To fit
the budget, narrow the roots or raise `max_bytes`.

## Your own document types

A document type is a profile file with `form = "document"`. It lists the
record types it shows, the relations it may follow, and its projections,
built from five blocks: a heading, a list, a table, a tree and a quote. Copy
`profiles/prd.toml` and change it. When the profile names a record type or
relation it does not declare, war refuses it and gives the rule. A document
type never claims, authorizes, verifies or resolves anything; that is a
Warrant's job. The full vocabulary is in `docs/TYPES.md`, under Projections.

Run `war render <projection> --json` to see where each line came from. Every
line traces to the record and revision it came from, or to the document or
the template.

The worked example in this repository is `docs/records/password-reset/`:
three record atoms, one `documents.toml`, and the four
documents in `generated/`.

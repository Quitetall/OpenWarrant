# Types: records and relations

A document's **type** (its profile, `profiles/<name>.toml`) says what it is
made of and what applies to it (OW-ADR-0031). This page covers the two
things a type composes, **records** and the **relations** between them, and
the **states** a record can be in.
`war impact` (below) is what they are for: one change, and everything it
reaches.

## Records

A record is the unit of meaning: `{id, type, body, source, revision}`.

- **id** — stable and global: `REQ-pr1`, `OUT-001`, `DEC-auth-2`. The
  grammar: an uppercase letter, then uppercase letters or digits, then one
  or more `-<letters or digits>` groups, at most 64 bytes. Unique across the
  program. Everything binds a record by its id, never by its file.
- **type** — a noun the governing profile declares (`requirement`,
  `outcome`, `constraint`, `decision`, `option`). The kernel knows two record
  types and no others: `obligation` (a Warrant's assurance atom holds them)
  and `item` (a ticket's checklist holds them), the two its capabilities
  compute on. Every other type is a profile's.
- **revision** — `sha256:` of the record's own bytes. Changing one record
  moves its revision and no other record's.

### Record atoms

Records live in record atoms, `docs/records/<area>/<NN>-<name>.md`, one
directory per area or feature. Every `*.md` in an area directory is read; a
file directly in `docs/records/` (a README) is not.

```markdown
---
schema: oh.war/records/v1
profile: delivery
---
# Password reset

## OUT-pr1 · outcome

A user who forgot their password regains access without support.

## REQ-pr1 · requirement
implements OUT-pr1

A reset token expires 15 minutes after issue.
```

- The frontmatter names the schema and the **profile that governs** every
  record in the file.
- A record opens at an unindented `## <ID> · <type>` line (the separator is
  ` · `, a middle dot between single spaces) and runs to the next one, or to
  the end of the file. Nested headings, blank lines and prose are part of it.
- Text between the frontmatter and the first record (a title, a paragraph)
  belongs to no record and moves no revision.
- A `## ` line with a `·` that is not that shape is refused, not read as
  prose: a record that silently failed to open would leave its text inside
  the record above it.

**Byte spans.** RC.2's unit-span rules
(`docs/sas/drafts/1.0.0-rc.2/format-contract.md` F2) are the reference, with
the heading as the marker: a record is the exact UTF-8 byte range `[start,
end)` from its heading line's first byte to the next heading's first byte (or
end of file), line endings and all, with no normalization. Inside a fenced
code block (three or more backticks or tildes, unindented) a heading or a
relation line is ordinary text; an unclosed fence is refused.

The worked example is `docs/records/password-reset/10-records.md`.

### Instruction sections: CLAUDE.md and AGENTS.md

The repository's own agent instructions are records too, read where they are
and never edited by the reader (M16). Each `##` section of the root
`CLAUDE.md` and `AGENTS.md` is a record of type **`instruction`**:

| | |
|---|---|
| **id** | `md:<file>#<slug>`: `md:CLAUDE.md#testing`, `md:AGENTS.md#tools` |
| **slug** | the heading as GitHub anchors it: lowercased; letters, digits, `-` and `_` kept; spaces become `-`; other characters dropped (`## Style & Lint` is `style--lint`). A repeated heading gets `-1`, `-2`, … in order; an empty one is `section` |
| **span** | from the heading line's first byte to the next `#` or `##` heading, the managed block's begin marker, or the end of the file; `###` headings, blank lines and line endings are inside it, nothing normalized |
| **revision** | `sha256:` of exactly that span, so an edit to one section moves that section's revision and no other |
| **governed by** | no profile: nobody governs a repository's instructions but the repository |

- Text before the first `##` heading, under a `#` heading, or between the
  managed block and the next heading belongs to no section.
- Inside a fenced code block a heading is text, as in a record atom; an
  unclosed fence runs to the end of the file and is not refused here.
- The **managed block** (`<!-- openwarrant:begin -->` …
  `<!-- openwarrant:end -->`, written by `war agents-md --block`) is no
  section and inside none, so restamping it moves no revision. A malformed
  block (two of them, or one that never closes) is a warning,
  `instruction.block-malformed`; the sections outside it are still read.
- **Nested files.** `[instructions] nested` in `openwarrant.toml` adds the
  files its globs match, such as `["**/CLAUDE.md"]` in a monorepo
  (`md:packages/api/CLAUDE.md#build`). A `**` walk skips hidden directories,
  symbolic links, `target/` and `node_modules/`. It is empty by default, so
  only the two root files are read unless you ask.

Cite a section like any record, from a relation line, an obligation's
`evaluates`, or a ticket item's `implements`:

```markdown
## CON-ci1 · constraint
constrains md:CLAUDE.md#testing
```

A Warrant's atoms or a ticket's text that name `md:CLAUDE.md#testing` are
found by `war impact` as documents citing it. The CLAUDE.md file itself is
never written to with the active Warrant's context: what changes lives in
`war prime`, and the file stays stable.

## Relations

A relation is `{from, kind, to}`, and `to` may pin a revision of its target
(`REQ-pr1@sha256:<64 hex>`).

### Kinds

- **Core**, a closed set the kernel computes from: `part_of`, `depends_on`,
  `implements` (intended coverage, never satisfaction), `constrains`,
  `evaluates` (an evaluation targets a record), `supersedes`, and the
  rationale edges of §35.7: `supports`, `refutes`, `trades_off_against`,
  `causes`, `qualifies`, `selected_over`.
- **Namespaced**, `<namespace>.<name>` (`x.mentions`, `contractor.bills`):
  carried, shown in the model, and **inert**. A namespaced relation drives no
  state, no readiness and no check, needs no declaration, and `war impact`
  lists it without walking it. A profile cannot give one kernel meaning.
- Any other word is refused (`record.relation-kind-unknown`).

### Where relations are authored

| Where | Form | Governed by |
|---|---|---|
| a record atom | a line `<kind> <target>[, <target>…]` inside a record, unindented, outside a fence | the atom's profile |
| a Warrant's assurance atom | `- **evaluates:** REQ-pr1@sha256:<hex>` under an `### OBL-…` heading | the Warrant's profile |
| a ticket item | `implements REQ-pr1` anywhere in the item's text | the ticket profile, and only where it allows `implements` |

A relation line's targets begin with an uppercase letter or `t-`: a record
id, a Warrant's own record (`OW-WAR-0148/OBL-001`), a ticket or an item
(`t-3f2a/i-9c01`); or with `md:` and hold a `#`: an instruction section
(`md:CLAUDE.md#testing`). A line without that shape is prose ("Tokens are
single-use.", "constrains everything" and "see md:CLAUDE.md for the rules"
are prose); a line with the shape whose target does not parse is refused.

**Pin what you judged.** An obligation's `evaluates` should pin the revision
it was written against: `war impact REQ-pr1` and `war model --json` show the
revision now. The pin is in the assurance atom, so the Warrant's contract
digest covers it and a signature over the contract covers which bytes were
judged. When the record changes, the verdict stays recorded, bound to the
revision it judged, and reads **stale**. Where the change also moves the
Warrant's reviewed subject, the verdict no longer counts for current work:
impact shows it `recorded` as it was written (`established`) and its
`verdict` now as `unknown`. An unpinned `evaluates` reads **unbound**
(nobody can say which bytes it judged).

## What a profile declares

Additive tables in `profiles/<name>.toml`; a file without them parses as
before and declares nothing.

```toml
[records]
types = ["outcome", "requirement", "constraint", "decision", "option"]

[relations]
allow = ["part_of", "depends_on", "implements", "constrains", "evaluates",
         "supersedes", "selected_over"]
# Every record of the first type has at least one relation of this kind to a
# record of the last type.
require = [["requirement", "implements", "outcome"]]
```

- `types` are lowercase words, each once; `obligation` and `item` are the
  kernel's and may not be declared.
- `allow` names core kinds only (a namespaced kind needs no declaration).
- `require` triples name a declared type (or a kernel type), an allowed
  kind, and a declared type.
- A bad declaration refuses the whole registry, `profile.records`.
- A core profile's file (`delivery.toml`) may declare a vocabulary: it is
  program data, not the core's fixed kind data, and loosens nothing an act
  reads. The built-in core profiles declare none, so a program without
  `profiles/` admits no record types until it says which.

This repository's `delivery.toml` declares the nouns of the password-reset
example; `ticket.toml` allows `depends_on`, `implements` and `part_of`.

## What `war check` refuses

Silent for a program with no record atom and no authored relation. Otherwise
`records.well-formed` names what was read, or, each by rule:

| rule | what |
|---|---|
| `record.malformed` | an atom that does not parse: frontmatter, schema, a heading-like line, a relation target, an unclosed fence |
| `record.profile-unknown` | the frontmatter names no profile of the program |
| `record.type-undeclared` | a type the governing profile does not declare, or a kernel type in a record atom |
| `record.duplicate-id` | an id declared again; the first declaration stands |
| `record.relation-kind-unknown` | a kind neither core nor namespaced |
| `record.relation-undeclared` | a core kind the governing profile does not allow |
| `record.relation-malformed` | an `evaluates` bullet naming no record id |
| `record.relation-required` | a record missing a relation its profile requires |
| `record.relation-target-unknown` | (a warning) a core relation whose target is no record of the corpus; kept, never dropped. A citation of an instruction section that does not exist (`md:CLAUDE.md#no-such`) is one |
| `instruction.block-malformed` | (a warning, said even with no record atom) an instruction file's managed block is doubled or never closes; its other sections are still records |

## States

A record's state is a fixed kernel set plus the refinements its profile
declares (OW-WAR-0148 M4; OW-ADR-0031). The fixed states map onto SAS §24 /
RQ-032's decomposition (phase, condition, outcome, currency, standing); they
name moments in it and replace nothing.

### The fixed set

Each is **computed** from facts the kernel already reads, or
**authenticated**: it holds only on the record of a human act or an ingested
independent verdict. A type reaches a state only when its profile selects
the capability that produces it. Several can hold at once.

| state | kind | capability | facet | holds for |
|---|---|---|---|---|
| `draft` | computed | `structure` | phase | a Warrant neither authorized nor resolved |
| `open` | computed | `claims` | phase | a ticket or item nobody has claimed or finished |
| `in_progress` | computed | `claims` | phase | an item claimed (it or its ticket) and not done; a ticket with a claim or a done item |
| `done` | computed | `claims` | phase | an item ticked; a ticket whose every item is |
| `accepted` | authenticated | `acceptance` | standing | a roadmap phase, once the roadmap record is accepted |
| `superseded` | computed | `links` | currency | a Warrant an authorized successor supersedes (OW-ADR-0022) |
| `authorized` | authenticated | `authorization` | phase | a Warrant whose authorization binds the contract as it compiles now |
| `verified` | authenticated | `verification` | outcome | an obligation an admissible independent verdict establishes |
| `resolved` | authenticated | `resolution` | phase | a Warrant whose §56.2 record binds the current contract |
| `achieved` | computed | `links` | outcome | a roadmap phase whose exit Warrant's resolution is recorded |

No profile adds a fixed state, and nobody enters one by hand.

### Declared states

```toml
# profiles/ticket.toml
[[states]]
name = "in_review"
refines = "in_progress"

# profiles/delivery.toml
[[states]]
name = "signed_off"
refines = "verified"
```

- `refines` names a fixed state; anything else is refused,
  `profile.state-refines-unknown`.
- A name that is a fixed state's is refused, `profile.state-collides`; a
  name that is not a lowercase word, or is declared twice,
  `profile.state-invalid`.
- Refining a state the profile's capabilities never reach (`signed_off` on
  the ticket profile, which has no `verification`) is refused,
  `profile.state-unreachable`.
- A core profile's file may declare states: program data, like its record
  vocabulary.

```text
war state <record> <name> [--note TEXT] [--as ACTOR] [--json]
```

enters one, as a `state.entered` event in the journal of the Warrant or
ticket that owns the record (`t-x/i-y`, `NS-WAR-0001/OBL-001`). Refused,
with nothing written:

| rule | what |
|---|---|
| `state.record-unknown` | no Warrant or ticket journal owns the record |
| `state.fixed` | the name is a fixed state's |
| `state.undeclared` | the record's profile declares no such state |
| `state.parent-not-holding` | the fixed parent does not hold for the record now; for an authenticated parent (`verified`), the act or verdict behind it must be on record first |

A declared state holds only while its parent holds — for an item's
`in_progress`, under the claim it was entered under — and reads **lapsed**
once the parent stops (an item finished, a claim released or re-taken). Of
two declared states refining the same parent, the later entry is the one on
record. Entering one again while it holds writes nothing.

A declared state **never satisfies a §56.1 requirement or a capability
gate**: nothing that evaluates one reads it. It is a qualifier a team can
see, never a way to read more than its parent.

### Where they show

- `war model --json`: each fixed state that holds, `kind: computed` or
  `authenticated` with its `facet`, and each declared state entered,
  `kind: declared` with `refines` and `lapsed`, beside the builders' own
  states (`phase`, `rung`, `currency`, `disposition`, `achieved`,
  `checklist`). An item's `in_progress` reads the claim locks, the one input
  outside the tree.
- `war show <ticket>`: ` [in_review]` (or ` [in_review, lapsed]`) after the
  ticket's state or an item's line; `war tickets`: the ones that hold.
- `war show <alias>` (`full_warrant` and `status` views): a "Declared
  states" section, only where the Warrant's journal holds one.

## The model

`war model --json` (`oh.war/model/v1`) carries record atoms' records beside
the corpus's own (Warrants, obligations, items, phases…), and their
relations beside the existing kinds, each with `to_revision` when it pins
one. Each instruction section is there too, type `instruction`, its source
the file that holds it. Every refusal above is also a model diagnostic under its rule, and an
unknown target is `model.relation-target-unknown`.

## `war impact <record>`

```text
war impact REQ-pr1 [--json]
```

Walks **incoming** relations from the record, transitively (what points at
it is what its change reaches; what it points at is not), and lists
(`oh.war/impact/v1`):

- **affected** records, each with the edge that reached it and its depth;
- **documents** that hold an affected record or name one by id — the
  Warrant whose basis names `REQ-pr1`, the ticket whose item implements it,
  the record atom that declares it; for an instruction section, the file
  that holds it (kind `instruction`) and every Warrant or ticket whose text
  cites `md:CLAUDE.md#testing`;
- **evaluations**: each obligation that `evaluates` an affected record, with
  its verdict now, its verdict as recorded, and whether it reads `current`,
  `stale` or `unbound`;
- **phases** the affected Warrants are placed in, whose progress is
  recomputed from them;
- **projections** known to include an affected Warrant or phase: its own
  `generated/` views, the corpus status and overview, the roadmap view; and
  exactly the declared document projections that **select** the record
  (scope `declared`, below), each with the affected records it also
  selects. A projection that only names the record by id is not listed:
  its bytes do not move when the record does.
- **inert**: namespaced relations into any of them, not walked.

An id that is no record of the model is refused, `impact.unknown-record`.
Nothing is written, and nothing is cleared: a stale verdict is a record of
what was judged, not a fault to delete.

## Projections

One set of records, many documents. A **document type** is a profile with
`form = "document"`: it composes records and declares **projections**, each a
pure function from a selection of the compiled model to bytes. Nothing of a
document is claimed, accepted, authorized, verified or resolved: its
capabilities are chosen from `structure` and `links` (the default), and any
other is refused, `profile.capabilities`. A Warrant manifest cannot name one;
the authority over a record, where it has any, is the Warrant that governs
it. `docs/DOCUMENTS.md` is the walk-through.

Four ship in `profiles/`, all rendering the same records:

| type | what it shows |
|---|---|
| `prd` | outcomes (quoted), requirements with the outcome each implements and what constrains it, constraints, non-goals, open questions |
| `architecture` | decisions with the alternatives each was chosen over (`selected_over`) and its consequences (`part_of` the decision), interfaces (a table), constraints; requirements named by id, not restated |
| `test-plan` | a coverage table, then each requirement and constraint with the obligations that `evaluates` it, their scope, evidence and verdict, and whether each judged the record's current revision or an earlier one (stale) |
| `agent-packet` | the records one stage needs from its root: the work, its outcome, the rules it must obey, the alternatives already rejected, consequences, interfaces, how it will be checked; each with source and revision; within a 16000-byte budget |

### Declaring a projection

```toml
schema = "oh.war/profile/v1"
name = "prd"
form = "document"

[records]                      # the nouns it shows; a record atom may be
types = ["outcome", "requirement", "non_goal"]   # governed by `prd` to hold them
[relations]
allow = ["implements", "constrains"]

[[projections]]
name = "prd"                   # unique across the program's document types
title = "{title}: product requirements"
intro = "…"                    # optional prose under the title
renderer = "markdown"          # or "json"
max_bytes = 16000              # optional budget; over it is refused, never truncated
sources = "Sources"            # the heading of the provenance table

[projections.select]
types = ["outcome", "requirement", "non_goal", "obligation"]
through = ["in:evaluates"]     # relations walked from the seeds
depth = 2                      # optional bound on the walk

[[projections.sections]]
heading = "Requirements"
intro = "…"
block = "list"
types = ["requirement"]
annotate = ["out:implements", "in:constrains"]
empty = "No requirement is recorded yet."
```

**Selection.** The walk starts at the subject's seeds — a document's
`roots`, or every record of its area — and follows `select.through`:
`in:<kind>` walks incoming relations, `out:<kind>` outgoing ones, a bare kind
both ways, and `in:<kind>:<type>` only to records of that type. Only records
of `select.types` are admitted, to `depth` steps.

**The template vocabulary.** Each section is one block over the admitted
records of its `types` (`of = "roots"` or `"rest"` narrows to the seeds or
the others), in authored order:

| block | renders | options |
|---|---|---|
| `heading` | the heading and its intro | — |
| `list` | a record list: `- **ID.** text` bullets, or a `### ID` block per record | `style = "bullets"\|"blocks"`, `show = "body"\|"summary"`, `annotate`, `provenance`, `fields` |
| `table` | records × fields | `columns`: `id`, `type`, `summary`, `revision`, `source`, `field:<name>`, `in:<kind>`, `out:<kind>`, each optionally `=<header>` |
| `tree` | each record with its children through named relations | `children = [{ relation = "out:selected_over", label = "…", empty = "…" }]`, `annotate`, `fields`, `show` |
| `quote` | each record's body, quoted, with its id | — |

An annotation reads the relation from the record's side: `out:implements`
"implements OUT-pr1", `in:constrains` "constrained by CON-pr1". Fields are
what a record carries beside its body: an obligation's `scope`, `evidence`
and `verdict` (read from the corpus), an item's `state`.

**Refused when the profile is read**, `profile.projection`: a record type the
profile does not declare (the kernel's `obligation` and `item` need no
declaration), a relation kind it does not allow, a namespaced kind (inert; a
projection may not give it meaning), a section type the selection does not
admit, an unknown block, column or renderer, a projection name used twice.

### What a rendering selects, exactly

A rendering **selects** a record when the record's bytes reach the output:
it is shown, or it authored an incoming relation an annotation shows (only
selected records' incoming relations are shown). Every rendering ends with a
provenance table of each selected record and its revision, so the bytes move
when a selected record changes and never when another does. A record named
only at the far end of an outgoing relation — the architecture view's
"Constrains REQ-pr1" — is **mentioned**, not selected. `war impact` lists
exactly the declared projections that select its record.

### Trace

`war render … --json` returns `oh.war/projection/v1`: the content, its
digest and size, the budget, `selects` (each record's id, type, revision,
source and the bytes rendered from it), `mentions`, and `trace` — every
output line in runs, each with its origin: `record` (id and revision),
`document` (the declaration's id and revision: the title) or `template` (the
document type and its profile file's digest: headings, intros, table
headers).

### Documents, `war render` and `war compile`

An area declares its documents in `docs/records/<area>/documents.toml`
(`oh.war/documents/v1`): each a `type`, optionally a `name` (default: the
type), `title` (default: the file's), `roots` (default: every record of the
area) and `max_bytes`. Its id is `<area>/<name>`.

```text
war render <projection> [--of <document | area | record>] [--max-bytes N] [--json]
```

prints the rendering and writes nothing. `war compile` writes each
declared document's projections to `docs/records/<area>/generated/`; `war
check --generated` renders them again and compares.

| rule | what |
|---|---|
| `documents.well-formed` | (pass) every declaration read, each of a document type |
| `documents.malformed`, `documents.type-unknown`, `documents.duplicate`, `documents.root-unknown` | a declaration refused |
| `projection.drift` | a generated projection differs from a fresh rendering (a hand-edit), or a file under `generated/` no document produces |
| `projection.missing` | a declared projection not yet compiled |
| `projection.compile` | a declared projection that cannot be rendered (over budget, an unknown root) |
| `projection.over-budget` | a rendering over its budget: refused by name with the records that cost the most; nothing is truncated |
| `projection.unknown`, `projection.of-missing`, `projection.of-unknown`, `projection.root-unknown` | `war render` was asked for something that is not there |

## From a sentence to records

One sentence in, typed records and the ticket that implements them out:

```bash
war create "Add password reset by email" --draft --records [--area password-reset-email]
```

or, to read the drafter's proposal before anything is written:

```bash
war plan "Add password reset by email" --records          # the request, for an agent of your own
war plan "Add password reset by email" --records --draft  # ask the drafter; validate; show it; write nothing
war plan --records --proposal <file> --reviewed --apply   # write it
```

The drafter is the one `war plan` already asks (`[plan] drafter_argv`). It
receives an `oh.war/records-request/v1` on stdin: the sentence, the area
when `--area` names one, the governing profile (`--profile`, default
`delivery`), that profile's record types, allowed and required relations,
`implements` for ticket items, and every record id the program already has.
It answers with an `oh.war/records-proposal/v1`:

```json
{
  "api_version": "oh.war/records-proposal/v1",
  "title": "Password reset by email",
  "area": "password-reset-email",
  "records": [
    {"id": "OUT-pre1", "type": "outcome", "title": "A user resets a forgotten password from an emailed link."},
    {"id": "REQ-pre1", "type": "requirement", "title": "The email is sent within a minute.",
     "body": "Markdown, optional.", "relations": [{"kind": "implements", "target": "OUT-pre1"}]}
  ],
  "ticket": {"title": "Add password reset by email", "body": "optional", "priority": 2,
             "items": [{"text": "Send the reset email", "implements": ["REQ-pre1"]}]}
}
```

Any other field is refused at parse. A proposal that asks
(`unresolved_questions`) instead of proposing is refused,
`plan.interview-required`, until `--answer <id>=<text>` answers it.

**Validated before anything is written, by the rules above.** The proposal
is rendered to the record atom `--apply` would write and read with every
atom on disk, by the same parser and the same rules `war check` applies:
`record.type-undeclared`, `record.relation-kind-unknown`,
`record.relation-undeclared`, `record.duplicate-id` (within the proposal and
against the corpus), `record.relation-required`. A relation, or an item's
`implements`, whose target is neither a record of the corpus nor one the
proposal makes is refused, `record.relation-target-unknown` (a warning in
`war check`; a proposal is not let in already dangling). A title or body
that would read as a heading or a relation line of its own is
`plan.record-body`. Every fault is named at once, and a refused proposal
writes nothing: not the atom, not the ticket, not a scratch file.

**What `--apply` writes.** `--apply` needs `--reviewed`
(`plan.review-required` otherwise, before the drafter is asked);
`war create --draft --records` is the fast path and applies directly. One
record atom, `docs/records/<area>/<NN>-<slug>.md` (the next free `NN`,
never overwriting), whose frontmatter names the profile and whose prelude
names the sentence; then a ticket through `war create` whose items read
`… (implements REQ-pre1)`, so `war model` carries each item's `implements`
and `war impact REQ-pre1` reaches the item. The ticket's body names the
records, their file and the proposal's digest. Applying the same proposal
again is refused, `record.duplicate-id` once per record, naming where each
already is. No drafter configured is `plan.no-drafter` (`ticket.no-drafter`
from `war create`), and nothing is invented.

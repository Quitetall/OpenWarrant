# Types: records and relations

A document's **type** (its profile, `profiles/<name>.toml`) says what it is
made of and what applies to it (OW-ADR-0031). This page covers the two
things a type composes, **records** and the **relations** between them, and
the **states** a record can be in.
`war impact` (below) is what they are for: one change, and everything it
reaches. The last section, "One Warrant, three encodings", covers the
Warrant itself: one noun, one id space, one list, and work brought in from
Beads, OpenSpec and Spec Kit.

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

## One Warrant, three encodings

A Warrant is one unit of work. Its minimum is a title; everything else is
optional, chosen by its type (OW-WAR-0148 M10). A "ticket" is a Warrant in
its light encoding; the word stays as that encoding's other name. A Warrant
is written down in one of three ways, and each reads forever, byte for
byte: nothing is migrated, and no signed digest moves.

| encoding | where | its type | written by |
|---|---|---|---|
| light | `docs/tickets/<t-id>/`: a manifest, an intent and a checklist | the working form (`profiles/ticket.toml`); its own `type` (bug, feature, ...) beside | `war create`, `war import` |
| directory | `docs/warrants/<alias>/`: a manifest and the atoms its profile requires | its profile: `delivery`, `decision`, ... | `war new`, `war plan --apply`, `war promote` |
| read in place | an OpenSpec change or a Spec Kit feature an `[[adapters]]` entry names | `openspec`, `speckit` | the tool that owns the folder; `war` never writes it |

There is no third file format. The light encoding is already the small
one, and an imported Warrant is a light one with one more manifest key,
`imported_from = "<format>:<id>"` (absent from every other, so a manifest
written before it reads and writes back unchanged).

### One id space

| id | names | for example |
|---|---|---|
| `t-<hex>`, `t-<hex>/i-<hex>`, `i-<hex>` | a light Warrant, one of its items | `t-3f2a`, `t-3f2a/i-9c01` |
| `<NS>-WAR-<NNNN>` | a directory Warrant | `OW-WAR-0148` |
| `openspec:<change>`, `openspec:<change>/<task>`, `openspec:<capability>#<requirement>` | a change read in place, its task, a requirement | `openspec:add-2fa/1.1` |
| `speckit:<feature>`, `speckit:<feature>/<T-id>`, `speckit:<feature>/<FR-, SC- or US-id>` | a feature read in place, its task, its record | `speckit:001-photo-albums/T004` |

The shape routes the id, so nothing is read to decide where it goes.
`war show <id>` and `war status <id>` take any of them: a light Warrant
renders as its document, a directory one as its §17.5 projection, one read
in place from its folder. `war impact` takes any record id of the model.

### The list

`war warrants` (also `war tickets`, `war ls`) lists every Warrant: the
light ones first, in the order work takes them (`result.tickets`, as
before), then the directory ones and the ones read in place
(`result.warrants`, each with `profile`, `encoding`, `state` and
`provenance`). A directory Warrant's state is the phase its journal
records (`provenance: recorded`): the list reads only its manifest and
journal, so it answers in milliseconds on a corpus of any size, and
`war status <alias>` is the computed answer. One read in place is `open`,
`in_progress` or `done` from its tasks (`computed`).

Filters read every encoding. `--type` takes a light Warrant's type (`bug`)
or a profile (`delivery`, `ticket`, `openspec`); `--state` the fixed three,
a declared state, or a recorded phase (`draft`, `authorized`, `resolved`),
a phase reading as one of the three (draft is open, authorized to verifying
is in progress, resolved is done); `--text` and `--search` read a Warrant's
title where they cannot read more. Labels and epics are the light
encoding's. A type that is neither, or a state that is none, is refused
(`ticket.filter-type-unknown`, `ticket.filter-state-unknown`).

`war ready` lists the items that can be claimed now, which are the light
Warrants'; `war next` adds the directory Warrants' acts and ready stages.
A Warrant read in place is never claimed here: its tasks are ticked in its
own files, by its own tool.

### Bringing work in

```bash
war import beads issues.jsonl      # Beads' issue JSONL, as `bd export` writes it
war import openspec .              # each change of openspec/changes/ (archive/ is not read)
war import speckit .               # each feature of specs/
war export beads > issues.jsonl    # every light Warrant, as Beads issue JSONL
```

Each imported change, feature or issue becomes a light Warrant whose
tasks are its items, worked with `war ready`, `claim` and `done` like any
other.

- **Deterministic.** An imported Warrant's id is `t-` and 8 hex of the
  sha256 of where it came from; its item ids come from that and each
  task's own key (`1.1`, `T001`); its UUID is a v7 from its source and its
  creation time. The same input gives the same files in every program.
  The journal line that records the import carries the time it ran, and a
  source that names no creation time takes the import's own.
- **Idempotent.** A Warrant whose `imported_from` is already here is named
  and left alone, so running an import again writes nothing.
- **All or nothing.** Every refusal is found before the first file is
  written; a refusal writes nothing.

| Beads (`internal/types/types.go`, `cmd/bd/export.go`) | the Warrant |
|---|---|
| `id` | `imported_from = "beads:<id>"`; `war export beads` gives it back |
| `title` | its title, and the text of its one item |
| `description`; `design`, `acceptance_criteria`, `notes` | its description; the other three as `## Design`, `## Acceptance criteria`, `## Notes from Beads` |
| `status` | `closed`: the item ticked, with `closed_at` and `close_reason` on its line; any other built-in status reads open, and the import says which |
| `priority`, `issue_type`, `labels` | priority (the same 0 to 4), type, labels, where the ticket profile admits them |
| `parent-child` | `part_of` the parent |
| `blocks` | the item waits on that Warrant (`after t-...`) |
| `comments` | dated notes |

An open epic with no blocking dependency gets no item and is worked
through the Warrants part of it. Derived fields (the counts,
`updated_at`, `parent`) are dropped, since an export computes them again;
who holds an issue now (`assignee`, `owner`, leases) is dropped with a
warning, since a claim says that here, locally. On export, a Warrant whose
items are not just its title is an issue with one child issue per item
(`<id>.<n>`, `parent-child`).

| OpenSpec (`openspec/changes/<change>/`) | the Warrant |
|---|---|
| `proposal.md` | the title (its `# ` heading, or the change's name) and the description (its first section, `## Why`) |
| `.openspec.yaml` `created:` | `created_at` |
| `tasks.md` | the items, ticked as ticked, each keeping its number |
| `specs/<capability>/spec.md` deltas | named in the description: what it adds, modifies, removes or renames |

| Spec Kit (`specs/<feature>/`) | the Warrant |
|---|---|
| `spec.md` | the title (`# Feature Specification: ...`), the description (`**Input**`), `created_at` (`**Created**`) |
| `tasks.md` | the items (`- [ ] T001 [P] [US1] ...`); `depends on T003` becomes `after` |
| `FR-` and `SC-` lines | records `SK<NNN>-FR-<n>` (requirement) and `SK<NNN>-SC-<n>` (outcome) in `docs/records/<feature>/10-speckit.md`, where the program's record format admits them; otherwise they stay in the description and the import says why (`speckit.records-not-admitted`) |

"Admits them" means the atom passes every rule `war check` applies to a
record atom: the `delivery` profile declares `requirement` and `outcome`,
and nothing it requires (a requirement's `implements` an outcome, say) is
missing. `.specify/` (templates, scripts, the constitution) holds no work
and is not read.

Refused by rule, each writing nothing:

| rule | what |
|---|---|
| `beads.malformed` | a line that is not an issue object, an issue with no id, title or `created_at` |
| `beads.not-an-issue` | a `_type` other than `issue` (a memory line) |
| `beads.status-unmapped` | a custom status |
| `beads.type-unmapped`, `beads.label-unmapped` | an issue type or label the ticket profile does not admit |
| `beads.dependency-unmapped` | a dependency other than `blocks` or `parent-child`, or a second parent |
| `beads.field-unmapped` | any other field that carries a value |
| `openspec.missing`, `speckit.missing` | a folder that is neither |
| `openspec.tasks-malformed`, `speckit.tasks-malformed` | a task line that is not one (`- [y]`, `- []`, no text), a number or id used twice, a Spec Kit task with no `T` id |
| `openspec.delta-malformed` | a requirement outside the four delta sections, a FROM with no TO |
| `openspec.requirement-duplicate`, `speckit.record-duplicate` | a requirement or record declared twice |
| `import.target-unknown`, `import.part-of-cycle`, `import.blocker-cycle` | a reference to nothing in the input or the program, or one that comes back to itself |
| `import.format-unknown` | a format other than `beads`, `openspec`, `speckit` |
| `export.beads-flags` | `war export beads` with a §68 flag |

### Read in place

A repository that keeps working in OpenSpec or Spec Kit names the folder
instead of importing it:

```toml
# openwarrant.toml
[[adapters]]
kind = "openspec"
path = "openspec"

[[adapters]]
kind = "speckit"
path = "specs"
```

`war warrants`, `war show`, `war status`, `war model`, `war impact` and
`war check` then read those folders on every call, with the readers the
import uses, and never write them. Each change or feature is a Warrant
(`change`, `feature` in the model), each task an `item` `part_of` it, each
requirement (OpenSpec's `### Requirement:`, Spec Kit's `FR-`), success
criterion (`outcome`) and user story (`story`) a record with the revision
of its own bytes. Relations: a change `implements` the requirements its
deltas add or modify (`openspec.removes` and `openspec.renames` are
namespaced: carried, inert); a Spec Kit task `implements` the story its
`[US1]` names and any `FR-` it names, and `depends_on` the tasks it says
it depends on. So `war impact openspec:auth-session#session-expiry` names
the change that modifies the requirement and that change's tasks.

`war status` prints a "Read in place" section (and `read_in_place` under
`--json`) only when an adapter is configured; the committed
`CORPUS_STATUS` projections never include it, since the folders are
another tool's and change without a compile.

Reported by rule: an entry that does not parse (`adapter.config`), a kind
this build does not read (`adapter.kind-unknown`), a path that is not there
(`adapter.path-missing`), an id no adapter reads (`adapter.unknown`), and
the readers' own rules above. `war check` makes each an error; the list,
the model and the status carry each as a warning beside what could be
read.

## Optional parts and the tick ladder

A Warrant's minimum is a title. Everything below is optional, and attaches
to any Warrant, a title-only one included (OW-WAR-0148 M13).

### Parts: tests, KPIs, milestones

```bash
war add t-3f2a --test "cargo test -p parser"            # the Warrant's own test
war add t-3f2a/i-9c01 --test "./smoke.sh" --name smoke  # one item's test
war add t-3f2a --kpi p95_ms --cmd "./bench.sh p95" --direction min --target 120
war add t-3f2a --milestone "Beta ships" --min observed  # an item with a minimum
war add t-3f2a/i-9c01 --min independent                 # make an item a milestone
```

- A **test** is a shell command (`sh -c`, from the repository root); its
  exit code decides pass or fail. Unnamed, it is `test-1`, `test-2`, ...
- A **KPI** is a command that prints one number, a direction (`max`: higher
  is better; `min`: lower is better), an optional `--target`, and a mode:
  `best` (the default: pass or fail against the target, and the best value
  kept), `threshold` (pass or fail only; needs a target), `optimise` (a
  signal only: it never decides a tick, and its best is kept). A KPI
  without a target decides nothing either.
- A **milestone** is an item that ticks a marker on the progress tracker,
  with an optional minimum level.

On a ticket (`t-x`) a test or KPI is the Warrant's and applies to every
item; on an item (`t-x/i-y`), or given with the text of a new item in the
same `war add`, it is that item's. Each part added is one line in the
ticket's optional checks atom and one `ticket.part_added` in its journal.
`war add` without a part is the item it always was.

### The checks atom

A ticket-encoded Warrant keeps its parts in `atoms/30-checks.md`, written
on the first part and found by that path (the manifest does not list it).
A ticket without it has no parts and reads, byte for byte, as before.

```markdown
# Checks

## Tests

- unit: `cargo test -p parser`
- smoke: `./smoke.sh` (for i-9c01)

## KPIs

- p95_ms: `./bench.sh p95` · min · target 120 · best
- coverage: `./cov.sh` · max · optimise

## Milestones

- i-77be · min observed
```

- A part is a list item in its section: `- <name>: <code span>`, then, for a
  KPI, ` · max|min`, optionally ` · target <N>`, and ` · best|threshold|optimise`;
  `(for i-x)` at the end scopes it to an item. The code span carries the
  command exactly (a command holding backticks takes a longer fence).
- A milestone is `- <item id>`, optionally ` · min <level>`.
- Prose, other headings and fenced blocks are a person's and are kept.
- Each line `war add` writes is appended to its section; nothing else
  moves. Edit the file by hand and the tool follows it.
- `war check` reports, by line, a malformed part (`checks.malformed`), a
  name used twice (`checks.duplicate`), and a part or milestone naming an
  item the checklist does not have (`checks.item-unknown`).

### The ladder: how a tick was earned

| level | earned by | written on the line |
|---|---|---|
| `claimed` | `war done`: the performer says so | `— done by claude, 2026-10-07` (as always) |
| `observed` | `war done --check`: the tests and KPIs that apply ran and passed; the receipt is journalled | `— done by claude, 2026-10-07 [observed]` |
| `independent` | `war verify <t-x/i-y> --response <file>`: a verdict by someone other than the performer, with evidence | `... [independent]` |
| `signed` | `war sign <t-x/i-y> --ssh-sign`: a human's signature over the item's text, verified | `... [signed]` |

`claimed < observed < independent < signed`. A claimed tick writes no
marker, so every checklist written before the ladder reads as claimed
ticks, and a parser that predates it reads a marker as part of the date.

**A marker is believed only as far as a record backs it.** `observed` needs
a passing check of the item in the ticket's journal (`ticket.item_done` or
`ticket.tick_raised` at that level, carrying each run's verdict, duration,
output digest and the commit); `independent` a `ticket.tick_verified` whose
verifier is not the tick's performer; `signed` a `ticket.tick_signed` whose
response verifies as a human's signature over the item's current text. A
marker nothing backs reads as the highest level that is backed (`claimed`
at the floor) and says why. So **a claimed tick never reads as checked,
verified or signed**, however its line was edited.

`war done <item> --check`:

- runs every test and every KPI that applies (the Warrant's, then the
  item's) through the gate runner, with its deadline; output lands under
  `.openwarrant/state/checks/<ticket>/`;
- ticks at `observed` only when every test passes and every KPI with a
  target meets it; a KPI that decides nothing is run and recorded;
- refuses, by name, when one fails (`ticket.check-failed`), and reads
  **UNKNOWN** (`ticket.check-unknown`) when one could not be established: a
  command that could not run, timed out, or (a KPI) printed no number.
  UNKNOWN is never a pass and never a fail. Nothing is ticked either way,
  the claim stays the agent's, and the round is journalled
  (`ticket.check_run`);
- refuses an item with nothing that could fail (`ticket.check-nothing`):
  observed needs an observation;
- on a done item, raises its tick to `observed` the same way, keeping who
  ticked it, when, and its note;
- composes with M11: `--if-rev` is compared in the same write, the agent's
  leases are renewed after each run, and its claim is checked again before
  the tick.

### Minimums

A type sets the least a tick must show, as data in its profile:

```toml
# profiles/ticket.toml
[ticks]
item = "claimed"          # every item (default: claimed)
milestone = "observed"    # every milestone

[ticks.types]             # a ticket's `type` (from [fields] types): its milestones
bug = "observed"
```

An item's minimum is the highest that applies: the profile's `item`; for a
milestone, also the profile's `milestone`, its ticket type's, and its own
`--min`. A tick below it is refused, `ticket.tick-below-minimum`, naming
the minimum, what set it, and the command that reaches it: `war done <item>
--check` for observed, the verification seam for independent, `war sign
<item> --ssh-sign` for signed. A level outside the ladder, or a type the
profile does not declare, is refused when the profile is read,
`profile.ticks`. No shipped profile declares `[ticks]`.

### KPIs over time

`war kpi run <t-x|t-x/i-y>` runs each KPI that applies, parses the one
number it prints (the whole output, or its last non-empty line), journals
every run as `ticket.kpi_run` (value, verdict, direction, target, mode,
commit), and says each one's latest, best and target. It ticks nothing. A
run short of its target is a warning, `kpi.target-missed`; a run that
printed no number, or exited non-zero, is UNKNOWN, `kpi.unknown`, and is
recorded with no value. `war show` gives the same standing for every KPI.

### Independent and signed

`war verify <t-x/i-y>` writes the request an independent verifier answers:
the item, its performer (who ticked it, or who holds its claim), the checks
that apply and the KPI runs on record. The answer is an
`oh.war/tick-verification-response/v1` (TOML or JSON) whose
`[verification]` is the same record a Warrant's verdicts are, admitted by
the same rule: no verifier (`tick.no-verifier`), no evidence
(`tick.no-evidence`), or a verifier who is the performer
(`tick.self-verification`) is refused, and so is a response naming another
performer than the record does (`tick.performer-mismatch`). An
`established` verdict ticks the item at `independent` (or raises its tick);
any other is journalled and raises nothing.

`war sign <t-x/i-y> --ssh-sign [--as <human>]` writes
`docs/authority/responses/<t-x>--<i-y>.signoff.response.toml` (the ticket,
the item, the sha256 of a statement of its text), signs it with the
human's key through the ssh agent, and records it only when the signature
verifies as that human's (`authority_check`, act `sign-off`). The tool
holds no key. Without `--ssh-sign` nothing is signed (`sign.ssh-required`);
`--dry-run` shows what would be signed and touches no key.

### Where every tick's level shows

| view | what it shows |
|---|---|
| `war show <ticket>` | each done item opens with its level, `(claimed)`, `(observed)`, `(independent: verified by X)`, `(signed: signed off by Y)`, and `; needs <level>` when below its minimum; an open milestone says what it ticks at; a **Checks** section lists tests, KPIs (latest, best, target) and milestones; `--json` items carry `tick` (`level`, `minimum`, `meets_minimum`, `written` and `unbacked` when a marker is not believed) |
| `war tickets` | `[ticks: 2 claimed, 1 observed]` at the end of a line with ticks; `ticks` in the JSON row |
| `war status` | a **Ticks** block after the corpus projection (never inside it, so working a ticket moves no generated file); `war status <ticket>` is the ticket's show |
| `war roadmap` | a **Milestones** section (in a program with a roadmap record): each milestone with the level its tick shows and whether it meets its minimum |
| `war board` | the same ticks and milestones |
| `war ui` | a badge per done item, a distinct word per level (claimed is drawn muted, never as a checked one; a tick below its minimum as a warning), and the milestones on the progress page |

### The create hint

`war create` prints one line suggesting `war add <id> --test "<command>"`:
a Warrant's smallest form is a title, and a test is what lets a tick be
observed. It never refuses anything. `[warrants] hints = false` in
`openwarrant.toml` turns it off.

### Harness bridges

- **The `TaskCompleted` hook** (`.claude/hooks/task-completed.sh`, in the
  plugin's `hooks.json`): when Claude Code marks one of its tasks completed
  and the task's subject or description names an item, it runs `war bridge
  claude-tasks --event - --apply`. It exits 0 on every path and writes
  nothing on stdout, so it never holds a task; a task naming nothing does
  nothing. Long-running checks run inside the hook's own timeout.
- **`war bridge claude-tasks`** reads a Claude Code task list: `--dir`,
  `--file`, or `~/.claude/tasks/$CLAUDE_CODE_TASK_LIST_ID/` (Claude Code
  keeps one directory per list there; its hook input names `task_id`,
  `task_subject` and `task_description`, code.claude.com/docs/en/hooks,
  "TaskCompleted"). Each completed task that names exactly one item
  proposes `war done <item> --check`, or `war done <item>` when the item
  has nothing to check. It prints what it would do and writes only with
  `--apply`, claiming an item nobody holds first. A tick it cannot make is
  a warning, `bridge.not-ticked`; the bridge never fails because a check
  did. The file layout inside a task-list directory is not documented by
  Claude Code, so it reads the documented field names (`id`/`task_id`,
  `subject`/`task_subject`, `description`/`task_description`,
  `status`/`task_status`) and names any other file UNKNOWN
  (`bridge.task-unreadable`) rather than guess.

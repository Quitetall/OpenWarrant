# OpenWarrant Format 1.0

| | |
|---|---|
| Status | **Draft** (`1.0.0-draft.2`). Directed by the owner 2026-10-11; not yet accepted. |
| Decision record | OW-ADR-0032 |
| Licence | Text: CC-BY-4.0 (`docs/format/LICENSE`). Grammar, schemas and examples: Apache-2.0 |
| Governance | `docs/format/GOVERNANCE.md` |

OpenWarrant Format is a plain-text format for engineering work: what is
wanted, why, what constrains it, and how it will be checked. People write
and edit it. Agents read it in small, cited pieces that a deterministic
command selects for them. This document is self-contained. The WAR
Software Architecture Specification (SAS) is rationale and history for it
and is not normative (Appendix C).

## 1. Scope and conformance

The key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD
NOT, RECOMMENDED, MAY and OPTIONAL are to be interpreted as in RFC 2119 and
RFC 8174 when, and only when, they appear in capitals.

Conformance classes:

- A **reader** parses documents into the model (section 8). It MUST refuse
  what this document says to refuse, with the rule id of section 11.2 and a
  `path:line`. It MUST NOT drop authored text without a finding.
- A **writer** produces documents. Every document it writes MUST be
  accepted by a conforming reader. It MUST preserve the bytes of records it
  does not change.
- A **checker** is a reader that evaluates rules and reports one result per
  rule: `PASS`, `FAIL` or `UNKNOWN`.
- A **context compiler** is a reader that produces context packets
  (section 10.3).

`UNKNOWN` means the question could not be decided: an input was missing, a
version was newer than the tool, or an extension was not understood. A
tool MUST NOT report `UNKNOWN` as `PASS` or as `FAIL`. A summary of several
results is `PASS` only when each is `PASS`.

This document defines the **core**. Optional profiles add to it (7.4). The
authority profile (authorization, independent verification, resolution,
signatures) and the federation profile are separate documents; a core tool
need implement neither.

## 2. Concepts

- **Document**: one UTF-8 Markdown file in this format: a frontmatter, an
  optional preamble, and units.
- **Unit**: a section or a record. Every unindented `## ` line outside a fence opens one.
- **Section**: a unit opened by `## <name>`. It is prose that answers one
  question (Intent, Basis…). In the model it is a record of type `section`.
- **Record**: a unit opened by `## <id> · <type> — <title>`. It has an id,
  a type, a title, fields, relations and a body. Everything that refers to
  a record refers to its id, never to its file.
- **Type**: a lowercase word naming what a record is. The core defines
  `section`, `obligation`, `item`, `milestone`, `stage`, `warrant` and
  `ticket`; other types come from the document's profile. A dotted type
  (`com.example.risk`) is an extension type (7.2).
- **Profile**: data (`oh.war/profile/v1`, TOML) naming the record types,
  relations, sections and frontmatter keys a document may or must have. In
  1.0 the keys and sections of the built-in profiles are the ones this
  document names (3.3, 4.1); declaring them in profile data is reserved for
  a later minor.
- **Relation**: `{from, kind, to}`, optionally pinning a revision of `to`.
- **State**: a value computed from records, journals and, under the
  authority profile, recorded acts. A state is never authored.
- **Work document**: a document that describes one unit of work (a
  Warrant). A ticket is a work document with the `ticket` profile.

## 3. Grammar

Appendix A is the grammar in EBNF; this section gives its meaning.

### 3.1 Encoding

A document is UTF-8. A reader MUST refuse invalid UTF-8 and NUL bytes
(`format.encoding`). LF and CRLF line endings are both accepted and are
preserved: no span is normalized. A leading byte-order mark is tolerated
and belongs to no unit.

### 3.2 Frontmatter

The first line is `---`; the frontmatter runs to the next line `---`. It is
a restricted subset of YAML, not YAML:

- `key: scalar`, at the top level;
- `key:` followed by lines `  - scalar`, a flat list;
- scalars plain, `"double-quoted"` (escapes `\"` `\\` only) or
  `'single-quoted'` (`''` for a quote);
- `#` comment lines and blank lines.

A reader MUST refuse (`format.frontmatter`), naming the construct: a
missing fence, nested mappings, anchors, aliases, tags, flow collections,
block or folded scalars, and duplicate keys. A structured value is a
scalar or a dotted key (`holder.kind: git`).

### 3.3 Frontmatter keys

| key | | meaning |
|---|---|---|
| `format` | REQUIRED | `openwarrant/<major>.<minor>` (section 6) |
| `profile` | REQUIRED | the profile that governs the document |
| `alias` | work documents | the repository-local id (`OW-WAR-0146`, `t-0d61d`). Its presence makes the document a work document |
| `uuid` | work documents | a UUIDv7 (section 5) |
| `summary` | work documents | one line, at most 200 bytes, saying what the work is for. RECOMMENDED elsewhere |
| `paths` | OPTIONAL | a list of repository globs the work touches (10.3) |
| `requires` | OPTIONAL | extension names a reader must understand (7.3) |
| `implements`, `depends_on`, `supersedes`, `part_of` | OPTIONAL | a target or a list of targets (3.6): relations of the document's own record, never fields |

A profile MAY declare further undotted keys (the built-in `delivery`
profile declares `assurance_level`; `ticket` declares `type`, `priority`,
`labels`, `created_at`, `created_by`, `imported_from`, `promoted_to`,
`issue`, `issue_url` and `due`). Any other undotted key is refused
(`format.key-unknown`, but see 7.2). A missing REQUIRED key, or one with an
empty value, is refused (`format.field-missing`) at line 1. A `summary`
over 200 bytes is a warning (`context.summary-shape`). The document's title
is its first `# ` heading; it is never repeated as a key.

### 3.4 Units

Outside a fence, an unindented line beginning `## ` opens a unit, which
runs to the next such line or the end of the file. Its **span** is that
byte range, `[start, end)`, heading and line endings included. Text after
the frontmatter and before the first unit is the **preamble**.

- A heading `## <id> · <type> — <title>` opens a **record**. The separators
  are U+00B7 MIDDLE DOT and U+2014 EM DASH, each between single spaces. The
  title is REQUIRED, one line, at most 120 bytes; an empty title or a
  longer one is refused (`format.title-missing`).
  A `## ` line containing ` · ` that does not match MUST be refused
  (`format.heading`); it is never prose.
- Any other `## <name>` heading opens a **section**. Its id is `S-` and the
  slug of the name: ASCII letters lowercased, digits kept, every other run
  of characters replaced by one `-`, leading and trailing `-` removed
  (`## Work order` is `S-work-order`). An empty slug is refused
  (`format.heading`). Its title is the name; it has no attribute block.
  Its body is the text after the heading line; one blank line directly
  after the heading is not part of it.

`###` and deeper headings are ordinary text inside a unit.

### 3.5 Attribute block

In a record, the lines directly after the heading, up to the first blank
line or the end of the span, are the **attribute block**. Each line is:

- a **field**, `<name>: <value>`; `<name>` an undotted word the type or
  profile defines, or a dotted name (section 7);
- a **relation**, `<kind> <target>[, <target>]…` (3.6);
- a **continuation**, a line beginning with two spaces, which belongs to
  the field above it.

A line whose first word is a core kind (3.6) or a dotted name followed by
a space is a relation line. Any other line is refused
(`format.attribute`). A field name appears once
per record (`format.field-duplicate`). A field's value is the text after
`: `, then each continuation with its first two spaces removed, joined by
LF; if the first line is empty, the value starts at the first
continuation. A REQUIRED field whose value is empty is missing. A reader
MUST NOT read a key, a relation or an item inside a
continuation. Everything after the blank line is the **body**; a record
with no fields or relations starts with a blank line.

### 3.6 Relations

Core kinds, a closed set: `part_of`, `depends_on`, `implements` (intended
coverage, never satisfaction), `constrains`, `evaluates`, `supersedes`,
`supports`, `refutes`, `trades_off_against`, `causes`, `qualifies`,
`selected_over`. A dotted kind (`x.mentions`) is an extension kind: carried
and inert. Any other kind is refused (`format.relation-kind`).

A target is an id (section 5), optionally pinned `@sha256:<64 hex>`. A
relation line whose target does not parse is refused
(`format.relation-target`). A target that names no record is a warning
(`format.target-unknown`) and the relation is kept; an extension kind's
target is not checked, since it may lie outside the repository. A profile names the
core kinds its records may use and the relations they must have.

### 3.7 Obligations

```markdown
## OBL-001 · obligation — a bundled gate run carries its output
scope: `war verify --bundle` on a scratch Warrant with one recorded run.
  No claim about bundles with several runs.
gate: gate://ops.conformance.plants@1.1.0
evaluates REQ-pr1@sha256:<64 hex>
part_of M-1
evidence:
  - the run entry's `stdout.sha256` equals the file's sha256;
  - with `max_excerpt_bytes` below the file size, `truncated` is true.
refusal: with the stdout file removed, `text` is absent, never "".
```

`<64 hex>` stands for a digest.

| field | | value |
|---|---|---|
| `scope` | REQUIRED | what the claim covers and, as far as known, what it does not |
| `evidence` | REQUIRED | the observation that would settle it |
| `refusal` | RECOMMENDED | an input the control is observed to reject |
| `gate` | OPTIONAL | `gate://<id>@<version>`, comma-separated |
| `scope_kind` | OPTIONAL | `single_instance`, `enumerated_set`, `bounded_domain`, `bounded_corpus`, `sampled_population`, `temporal_window`, `existential`, `universal`, `formal_model` |
| `criticality` | OPTIONAL | `required` (default) or `advisory` |

A missing required field is refused (`format.field-missing`). Without
`refusal`, a checker SHOULD warn (`format.refusal-missing`): a claim that
nothing was seen to reject is satisfiable by a control that accepts
everything. `disposition`, `verdict`, `state` and `status` MUST NOT be
authored on an obligation (`format.authored-state`). A disposition comes
only from a recorded verdict, and a verdict whose verifier is the work's
performer does not establish it. A `### OBL-` heading inside a Format 1.0
document is prose; a checker SHOULD warn (`format.legacy-obligation`).

### 3.8 Items

An item is a list line in a unit's body, outside a fence and a
continuation:

```markdown
- [ ] Email template (i-4d11)
- [ ] Docs (i-77be, after i-4d11)
- [x] Reset endpoint (i-9c01) — done by claude, 2026-09-25: POST /reset
```

Optional indentation, `-` or `*`, a box `[ ]` (open) or `[x]`/`[X]`
(done), the text, a group `(<item-id>[, after <ref>]…)`, and, on a done
item, optionally ` — done by <actor>, <YYYY-MM-DD>` and `: <note>`.
`after` names an item, a ticket, or another ticket's item. `implements
<id>` in the text is a relation. `[]`, `[y]` and other boxes are refused
(`format.item`). A box line with no id group is reported
(`format.item-id-missing`); a writer assigns an id. A group that is not at
the end of the line is no id group. Item ids are unique in their document
(`format.id-duplicate`).

In the model an item is a record of type `item`: its title is its text
without the id group and the `done` suffix, `fields` is `{}`, `body` is
`""`, each `after` is a `depends_on` relation, and open or done is its
`checklist` state, never a field.

### 3.9 Fences

An unindented line of three or more backticks or tildes opens a fence; the
same character, at least as many, followed only by spaces or tabs, closes
it. A backtick fence's info string contains no backtick. Inside a fence
there are no units, fields, relations or items. An unclosed fence is
refused (`format.fence-unclosed`).

## 4. Work documents

### 4.1 The single-file Warrant

A Warrant is one file: `docs/warrants/<alias>.md`, or
`docs/warrants/<alias>/warrant.md`; a ticket `docs/tickets/<alias>.md` or
`docs/tickets/<alias>/ticket.md`. Both forms of one alias is refused
(`format.document-duplicate`).

```markdown
---
format: openwarrant/1.0
profile: delivery
alias: OW-WAR-0146
uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e
summary: A verification bundle carries what each gate printed.
assurance_level: basic
---
# A verification bundle carries what each gate printed

## Intent
The blind verifier decides from the bundle alone…

## Basis
…
## Work order
…
## Assurance
## OBL-001 · obligation — a bundled gate run carries its output
…
```

The document's own record has the alias as id, type `ticket` under the
`ticket` profile and `warrant` otherwise, the `# ` heading as title, the
frontmatter as fields (except `format` and the relation keys of 3.3, and
dotted keys, which go to `ext`) and the preamble without its `# ` line as
body. Its sections, records and items are scoped to it (section 5), and
each is `part_of` it. That containment relation is in the model but not in
any `content_digest` (section 9).

The profile names the required sections (`format.section-missing`). Built
in: `delivery` requires Intent, Basis, Work order and Assurance and at
least one obligation; `decision` requires Intent and Basis; `ticket`
requires Checklist. A section answers: **Intent** the problem, the outcome
wanted and what is out of scope; **Basis** sources, prerequisites and
unknowns; **Work order** deliverables, frozen surfaces, autonomy limits and
rollback; **Milestones** the records below; **Assurance** the obligations;
**Checklist** the items.

A **milestone** is a record of type `milestone` (`## M-1 · milestone —
…`). A **stage** is a record of type `stage`, `part_of` its milestone, with
fields `executor` (e.g. `agent://claude-code`), `tier` (`T0`–`T3`),
`budget_tokens` (integer) and `context` (comma-separated ids). An
obligation is `part_of` the milestone it gates.

### 4.2 The manifest is optional

Composition is inferred from the file: the units, in order. A
`manifest.toml` MAY sit beside `warrant.md` for tool settings. It MUST NOT
list atoms and MUST NOT carry a core key (3.3) or a key the profile
declares (`format.manifest-restates`): nothing is said twice.

### 4.3 Evidence lives outside

Journals, gate runs, receipts, verification bundles, signatures and
generated files live beside the document (`docs/warrants/<alias>/`), are
not authored, and are never needed to read the document's meaning. A
reader MUST produce the model of a work document from the document alone;
what those files add is state.

### 4.4 Record documents

A document that is not a work document is any `docs/records/<area>/*.md`
whose frontmatter declares `format:`. It has no alias and no required
sections. Its own record is `doc:<path>`, type `document`, with the
frontmatter (except `format`) as fields, dotted keys in `ext`, the `# `
heading as title and the preamble without it as body. Its records are
global and each is `part_of` it, outside `content_digest`.

## 5. Identity

- A **record id** is an uppercase letter, then uppercase letters or digits,
  then one or more groups of `-` followed by letters or digits; at most 64
  bytes: `REQ-pr1`, `OBL-001`, `S-intent`.
- An **item id** is `i-` and 3 to 16 lowercase hex digits. A bare item id
  names an item of the same document.
- In a work document, ids are local and the global id is `<alias>/<id>`
  (`OW-WAR-0146/OBL-001`, `t-0d61d/i-de0e`). Records of other documents
  are global. Every id is unique in its scope (`format.id-duplicate`).
- Other ids: an alias (`OW-WAR-0146`, `t-0d61d`); `war://<uuid>`;
  `md:<file>#<slug>` (a `##` section of `CLAUDE.md` or `AGENTS.md`);
  `doc:<path>` (a document no type reads). The shape routes the id.
- A work document's `uuid` is a UUIDv7 and never changes. A reference that
  must survive renaming SHOULD use `war://<uuid>`. A relation's `to` is the
  target as authored, a local id made global; a `war://` target is never
  rewritten to an alias, in the model or in a digest.
- Legacy stores (section 12) keep their ids unrenamed, including shapes
  this section does not produce (`OW-WAR-0146/M1`, `WAR-SAS-101.1`).
- A unit's **revision** is `sha256:` of the exact bytes of its span; an
  item's, of its line with its terminator. Editing one unit moves its
  revision and no other.

## 6. Versioning and compatibility

The version is declared in every document: `format: openwarrant/1.0`. A
document declaring `1.N` uses only what versions up to `1.N` define.

- **Patch** (1.0.1): wording only. No rule changes.
- **Minor** (1.1): additions only — optional keys and fields, types,
  profiles, relation kinds. A document valid under `1.N` is valid under
  `1.N+1` with the same model and digests.
- **Major** (2.0): anything else, under a new `format` value. Every major
  is read forever. Nothing is migrated in place; no recorded digest is
  recomputed.

| the document declares | the reader |
|---|---|
| a newer major | reads nothing; every check `UNKNOWN` (`format.newer-major`) |
| a newer minor | reads under section 7; a unit it cannot parse is `UNKNOWN` (`format.newer-minor`), not refused |
| no `format` | reads it as a legacy encoding (section 12) or refuses it (`format.unrecognised`). A document whose first line is not `---` declares no format |

JSON schemas follow the same rule: `oh.war/<name>/v<major>`, additive
within a major.

## 7. Extensions and unknown names

### 7.1 Names

A **dotted name** (`x.note`, `kf.enterprise_id`, `com.example.risk`) is an
extension name. The prefix `x` is private. Other prefixes MUST be a
reverse domain name or a prefix in the registry (Appendix C). An undotted
name belongs to this format or to the governing profile.

### 7.2 Must-ignore

A reader MUST ignore an extension key, field, relation kind, type or
section it does not understand, and MUST carry it in the model (`ext`,
section 8). An undotted name neither this format nor the profile defines
is refused (`format.key-unknown`), unless the document declares a newer
minor than the reader's; then it is ignored and reported `UNKNOWN`. This
catches typos without blocking later versions.

### 7.3 Ignoring never allows

Ignoring a name MUST NOT turn a refusal into an allow. A checker MUST NOT,
because it ignored something, report `PASS` where it would otherwise report
`FAIL` or `UNKNOWN`. An extension whose absence would make any check more
permissive is **critical**: the document MUST list it in `requires`, and a
reader that does not understand a listed name MUST report every check over
that document `UNKNOWN` (`format.requires-unknown`).

### 7.4 Profiles

A profile MAY narrow the core. It MUST NOT loosen a core rule, change the
meaning of a core type, kind or key, or give an extension kind core
meaning. The federation profile (Knowledge Fabric, Katana, Liminal, BLUT)
uses the prefixes `kf`, `katana`, `liminal` and `blut`. Its identifiers,
including `kf.enterprise_id`, are not core.

## 8. The canonical JSON model

A reader produces one model, `oh.war/model/v1`. It carries content.

```text
model      = { schema, format, basis_digest, records[], relations[], states[], diagnostics[] }
record     = { id, type, title?, document?, section?, source, line, span,
               revision, content_digest, fields, body, ext, governed_by? }
relation   = { from, kind, to, to_revision? }
state      = { record, kind, value, provenance, facet?, refines?, lapsed? }
diagnostic = { rule, record, message }
```

| member | value |
|---|---|
| `schema` | `"oh.war/model/v1"` |
| `format` | the highest format version the reader implements, `"openwarrant/1.0"` |
| `basis_digest` | `sha256:` of the RFC 8785 bytes of the array of `[id, revision]` pairs, in record order |
| `id`, `type` | as authored; global ids (section 5) |
| `title` | the heading's title; absent only for legacy records with none |
| `document` | the global id of the work document that holds the record; absent otherwise |
| `section` | the id of the last section opened before the record in its document; absent if none |
| `source`, `line`, `span` | repository-relative path with `/`; 1-based line of the heading; `[start, end]` byte offsets. A legacy record with no span of its own has the whole file's |
| `revision`, `content_digest` | section 9 |
| `fields` | object: name → string, or array of strings for a frontmatter list. `{}` when none |
| `body` | the body text, exactly; `""` when none |
| `ext` | object of dotted names → value; `{}` when none |
| `governed_by` | authority profile; absent otherwise |

`records` are sorted by `id`; `relations` by `(from, kind, to,
to_revision)`; `states` by `(record, kind, value)`; `diagnostics` by
`(rule, record, message)`; all comparisons by UTF-8 bytes. Absent optional
members are omitted, never `null`. The model is serialized as RFC 8785
JSON. A consumer MUST ignore members it does not know. Relation kinds
outside 3.6 (`roadmap`, `parent`, `promoted_to`) occur only in records
lowered from legacy stores.

## 9. Digests

- SHA-256, written `sha256:<64 lowercase hex>`. JSON is canonicalized per
  RFC 8785. A digest over a JSON value hashes `{"digest_domain": "<domain>",
  "payload": <value>}` with domains named `oh.war/<name>/v1`.
- **revision** — over exact bytes (5). It is what an existing signature
  binds. It never changes meaning.
- **content_digest** — over meaning. Domain `oh.war/record-content/v1`;
  payload `{id, type, title, fields, relations, body}` where `id` is the
  global id, `title` is `""` when absent, `relations` is the sorted array
  of the record's outgoing `{kind, to, to_revision?}` without the
  containment `part_of` of 4.1 and 4.4, and `body` has CRLF
  replaced by LF, trailing spaces and tabs removed from each line, and
  trailing empty lines removed. The same record has the same
  `content_digest` in every encoding (section 12).

A new signature SHOULD bind both. A digest computed under 1.0 MUST
recompute identically under every later 1.x.

## 10. Agent context

### 10.1 Summaries

Every record has a one-line title; every work document a `summary`. These
are the summary tier: an agent reads them before deciding what to read in
full. Titles and summaries SHOULD say what the thing is, not where it is.
An authored record SHOULD cite records by id, not by section number of
another document, and SHOULD be understandable from its own text and the
records it relates to.

### 10.2 The index

A tool that serves agents MUST be able to generate an index,
`docs/generated/llms.txt` by default, in the style of llms.txt:

```text
# <repository name>

> OpenWarrant index (openwarrant/1.0). Generated; do not edit. Read any line's record with `war context <id>`.

## <source path>
> <document summary, or ->
- <id> · <type> · <state> · <title> · <source>:<line>
```

Sources sorted by path, records by line. A work document is listed by its
own record alone; its units are one `war context <alias>` away. Every
record of any other document is listed. The `> ` line is the work
document's summary, or `-`. `<state>` is the record's **primary state**:
the value of its state of kind `phase`, else `checklist`, else
`disposition`, else `-`. A record with no title shows its first non-blank
body line that is not one of its relation lines, cut at 120 bytes on a
character boundary; `…` is added only when it was cut. With neither, the
title is `-`.

### 10.3 The context compiler

`war context <target> [--budget <bytes> | --tokens <n>] [--depth <d>] [--json]`

emits the context an agent needs for a target, as a pure function of the
documents. Identical arguments and identical input files MUST produce
identical output bytes. It MUST NOT read clocks, the environment, claim
locks, anything outside the working tree or any path the repository's
version control ignores, and MUST NOT depend on directory order. `--tokens n` means `--budget 4n`. Defaults: budget 16000
bytes, depth 3.

**1. Roots.** By the target's shape: an item id, the item, and at
distance 1 its document's `S-intent` (without one, the document's record),
reached by the step `item-intent` from the item; a record or section id,
that record;
an alias or `war://<uuid>`, the work document's record; `md:…`, that
section; a path of a document, its work document record, or else all its
records; any other path, every work document whose `paths` glob matches
it. None is refused (`context.target-unknown`). For a work document root,
every unit and item of the document is also at distance 0; the roots that
step 5 measures are the records this step names, not those units.

**2. Walk.** Breadth-first from the distance-0 set, up to the depth, over
these steps, in this order:

| step | follows |
|---|---|
| `out:implements` | what the work is for |
| `out:depends_on` | what it waits on |
| `in:constrains` | rules that constrain it |
| `out:selected_over` | alternatives already rejected |
| `in:evaluates` | obligations that check it |
| `out:part_of` | its milestone, phase or parent |
| `in:part_of` from a `consequence` | consequences to respect |
| `in:implements` from an `interface` | interfaces |

`out` follows a relation from the record; `in`, to it. Only core kinds are
walked. A record's **distance** is the level that first reached it, and
its **via** the first `(step, from)` that did, by step order and then by
`from` id. Unknown targets are skipped with a diagnostic. This walk is
named `core`. A profile MAY declare another walk; the packet names the
walk it used.

**3. Order.** By distance, then type rank — `warrant`/`ticket`, `section`,
`item`, `requirement`, `outcome`, `constraint`, `decision`, `interface`,
`option`, `consequence`, `obligation`, `milestone`, `stage`, `instruction`,
any other — then source path, then line.

**4. Render.** Markdown, LF line endings:

```text
# Context: <target>
<!-- oh.war/context/v1 · budget <B> · depth <d> · walk <name> · selection <selection_digest> -->

## Index
[F] <id> · <type> · <state> · <title> · <source>:<line>
[S] <id> · <type> · <state> · <title> · <source>:<line>

## Records
> <id> · <type> · <source>:<line> · sha256:<first 12 hex of revision>
<the unit's exact span, with a final LF added if missing>

```

`selection_digest` is the `basis_digest` rule (section 8) over the
selected records. In the index, `[F]` marks a record whose full text
follows and `[S]` one given only by its summary. `<state>` is the primary
state of 10.2, or `-`. An item whose section is given in full is marked
`[F]` and has no entry of its own under Records: its line is already
there. An item that is the target has its own entry.

**5. Budget.** Bytes are UTF-8 bytes of the Markdown rendering. If the
header and index alone, or with the full text of every root, exceed the
budget, the compiler MUST refuse (`context.over-budget`), naming the bytes
needed, and emit nothing. Otherwise records are given in full in order
until the next one would not fit; it and every later record are `[S]`. A
record is never cut. An `[S]` line is the agent's cue to run `war context
<id>`.

`--json` emits `oh.war/context/v1`, the same selection: `{schema, target,
budget, bytes, depth, walk, selection_digest, records[{id, type, title,
state, source, line, revision, distance, via?{step, from}, tier, text?}],
diagnostics[]}`, `tier` being `full` or `summary` and `text` the exact
span for `full` (an item's line). `title` is always present, `-` when the
record has none; `diagnostics` have the model's shape. `bytes` is the size
of the Markdown rendering. A tool MAY wrap the packet in its own report
envelope; the packet is unchanged inside it.

## 11. Conformance

### 11.1 The corpus

`conformance/format/` holds one directory per case, derived from this
document's text and examples, never from a tool's output:

```text
conformance/format/<NNN>-<slug>/
  case.toml            # section = "3.7", says = "<the sentence tested>"
  input/**             # a repository tree: documents and profiles/
  args                 # context cases: the arguments, one line
  expected/model.json  # the RFC 8785 model, or
  expected/refusal     # one line: <rule id> <path>:<line>, or
  expected/unknown     # one line: <rule id>, or
  expected/context.md  # context cases: the exact output
```

Exactly one of `model.json`, `refusal`, `unknown` and `context.md` is
present. A conforming tool passes a case when its output equals the
expected bytes. `model.json` holds the compared subset of the model that
`conformance/format/README.md` defines, after RFC 8785 canonicalization,
its `diagnostics` compared as a set of `(rule, record)`. A context refusal
is one line, `<rule id>`. Every refusal rule of 11.2 has at least one
case, and every claim has a case that is refused.

### 11.2 Rule ids

Refusals: `format.encoding`, `format.frontmatter`, `format.key-unknown`,
`format.heading`, `format.title-missing`, `format.attribute`,
`format.field-duplicate`, `format.field-missing`, `format.authored-state`,
`format.relation-kind`, `format.relation-target`, `format.item`,
`format.id-duplicate`, `format.fence-unclosed`, `format.section-missing`,
`format.document-duplicate`, `format.manifest-restates`,
`format.unrecognised`, `context.target-unknown`, `context.over-budget`.
Unknown: `format.newer-major`, `format.newer-minor`,
`format.requires-unknown`. Warnings: `format.target-unknown`,
`format.refusal-missing`, `format.item-id-missing`,
`format.legacy-obligation`, `context.summary-shape`,
`context.span-unknown`.

A finding about a whole document (a missing fence, key or section, a
restated manifest key) is reported at line 1; `format.document-duplicate`
at `<alias>.md:1`. A reader reports each fault once: a unit it refused is
not also reported missing.

## 12. Legacy encodings (non-normative)

Stores written before this format are read forever, byte for byte. Nothing
is migrated and no signed digest moves. Each lowers to the same model; a
legacy unit keeps the revision of its legacy bytes and has the
`content_digest` of its Format 1.0 equivalent.

| encoding | lowers to |
|---|---|
| `oh.war/records/v1` record atom | records without titles; spans run to the next record heading, as before; relation lines anywhere in a record |
| directory Warrant: `manifest.toml`, `atoms/*.md`, `45-milestones.yaml`, `deliverables.toml` | `uuid`, `local_alias` (→ `alias`), `title`, `profile` and, when present, `assurance_level` → document fields; intent, basis and work order atoms → `S-intent`, `S-basis`, `S-work-order`, any other atom role → `S-<role>` (`S-<role>-2` on a repeat); milestones (ids kept, `<alias>/M1`) and stages → records; each `### OBL-NNN — statement` block → an obligation, its `- **key:**` bullets → fields; each deliverable → a `deliverable` record with its `[[…]]` span, a title and a `target_ref` field, and the deliverables' `target_ref`s are the document's `paths`; `[[implements]]` → `implements`; `[[supersedes]]` → `supersedes`; `[[parents]]` → `parent`; `[[roadmap]]` → `roadmap`; `enterprise_id` → `kf.enterprise_id`, omitted when empty |
| ticket, `oh.war/ticket/v1` | a work document, profile `ticket`; manifest keys → document fields; `promoted_to` → a relation; `15-checklist.md` → `S-checklist`, lines unchanged |
| SAS document and revisions, `docs/sas/` | a `spec` record; sections and requirement rows → records with their ids (`WAR-SAS-101.1`) |
| ADR atom | one record, id the alias, type `adr`; `adr_uuid` → `uuid`; `status` → field; `governs`, `supersedes` → relations |
| roadmap, `docs/roadmap/` | a `roadmap` record; each phase a `phase` record `part_of` it, with `depends_on` |

Legacy kinds and ids are carried unrenamed, so existing output and
signatures stand. A legacy work document has no `summary`; its document
record has the `content_digest` of a Format 1.0 equivalent with the same
keys, and every other unit has its equivalent's.

## 13. Changes from current practice

Each is deliberate (OW-ADR-0032).

1. Every document declares its version (`format:`).
2. A Warrant is one file with optional manifest, not a manifest plus five
   atoms; nothing is restated between them. Evidence stays beside it.
3. Any `## ` line opens a unit; a plain `## Name` is a section with id
   `S-<slug>`. In `records/v1` a plain `##` stayed inside the record above.
4. A record heading carries a REQUIRED title after ` — `. A `records/v1`
   reader refuses it, which is the safe failure.
5. Records have fields, in an attribute block; relation lines belong there.
6. An obligation is a record, not a `### OBL-` heading with bold bullets.
   Continuations are verbatim (DV-14). `refusal` is a field. `disposition`
   is no longer authored.
7. A ticket is a work document with the `ticket` profile.
8. An unknown undotted key is refused (today kept silently, DV-3).
9. `requires` marks critical extensions.
10. `enterprise_id` and federation leave the core.
11. The model carries content and a `content_digest` over meaning.
12. A required summary tier, a generated `llms.txt` index, and a
    deterministic context command.
13. The SAS is rationale. Records cite ids, not `§`.

## Appendix A. Grammar (normative)

EBNF over lines. `SP` is one space, `EOL` is LF or CRLF. Fences (3.9) are
recognised first; nothing inside one matches the rules below.

```ebnf
document      = [ BOM ] frontmatter { preamble-line } { unit } ;
frontmatter   = "---" EOL { fm-line EOL } "---" EOL ;
fm-line       = fm-pair | fm-listhead | fm-item | comment | blank ;
fm-pair       = name ":" SP scalar ;
fm-listhead   = name ":" ;
fm-item       = SP SP "-" SP scalar ;
name          = word | dotted ;
word          = lower { lower | digit | "_" | "-" } ;
dotted        = label "." label { "." label } ;
label         = ( lower | digit ) { lower | digit | "_" | "-" } ;
scalar        = plain | dq-string | sq-string ;
plain         = plain-start { char } ;  (* no ": " and no " #" inside *)
plain-start   = char - ( "[" | "{" | "&" | "*" | "!" | "|" | ">" | "'"
                | '"' | "%" | "@" | "`" | "-" SP ) ;

unit          = record | section ;
section       = "## " section-name EOL { line EOL } ;
section-name  = text - ( text " · " text ) ;
record        = record-head EOL attributes [ blank EOL body ] ;
record-head   = "## " record-id " · " ( word | dotted ) " — " title ;
record-id     = upper { upper | digit } ( "-" alnum { alnum } )+ ; (* ≤ 64 bytes *)
title         = text ;                                              (* ≤ 120 bytes *)
attributes    = { ( field | relation-line | continuation ) EOL } ;
field         = name ":" [ SP text ] ;
continuation  = SP SP text ;
relation-line = kind SP target { "," SP target } ;
kind          = word | dotted ;
target        = ref [ "@sha256:" hex64 ] ;
ref           = record-id | alias "/" ( record-id | item-id ) | alias
              | item-id | "war://" uuid | "md:" path "#" slug | "doc:" path ;
alias         = record-id | "t-" hex { hex } ;
body          = { line EOL } ;

item          = { SP } ( "-" | "*" ) SP box SP text SP "(" item-id
                { "," SP "after" SP item-ref } ")" [ done ] ;
box           = "[ ]" | "[x]" | "[X]" ;
item-id       = "i-" 3*16 hex ;                 (* lowercase *)
item-ref      = item-id | alias [ "/" item-id ] ;
done          = " — done by " actor ", " date [ ": " text ] ;
gate-uri      = "gate://" gate-id "@" semver ;
gate-id       = label { "." label } ;
semver        = digit+ "." digit+ "." digit+ [ "-" text ] ;  (* SemVer 2.0.0 *)
actor         = text - ( text ", " text ) ;
date          = digit digit digit digit "-" digit digit "-" digit digit ;
path          = (* a repository-relative path, "/"-separated, no ".." *) ;
slug          = lower-or-digit { lower-or-digit | "-" } ;
uuid          = (* RFC 9562 text form, lowercase *) ;
hex64         = 64 hex ;  hex = digit | "a" … "f" ;
```

The 120- and 200-byte limits and the 64-byte id limit count UTF-8 bytes; a
JSON schema can only approximate them in characters.

## Appendix B. Divergences DV-1 to DV-14

`docs/GRAMMAR.md` lists where `war` and the SAS text disagree. "Text":
this format now says what `war` does. "war": the tool changes.

| DV | divergence | resolution | changes |
|---|---|---|---|
| 1 | SAS example nests `holder:`; `war` refuses nested maps | no nested mappings (3.2); `holder` is a federation dotted key | text |
| 2 | frontmatter is a YAML subset | the subset is normative (3.2, Appendix A); it is not YAML | text |
| 3 | unknown undotted header key kept silently | refused unless a newer minor is declared (7.2) | war |
| 4 | unknown manifest key ignored | manifest optional, atoms never listed, no restated key (4.2); legacy manifests read as today | war, for the new form |
| 5 | obligation Markdown form is tool practice only | obligation record (3.7) is normative; `### OBL-` is legacy | both |
| 6 | `atom_uuid` in the example, in no atom | gone; units have ids, documents a `uuid` | text |
| 7 | header must restate manifest; SAS silent | one frontmatter; nothing to restate. `war` keeps its check on legacy atoms | neither |
| 8 | canonical JSON structured atoms refused | no structured atoms; milestones are records; JSON is output, never authored | text |
| 9 | role `adr` vs section `decisions` | no roles; legacy role stays `adr` | text |
| 10 | unknown undotted optional role refused | unknown undotted names refused (7.2), as `war` does | text |
| 11 | `gate://id/version` vs `gate://id@version` | `gate://<id>@<version>` | text |
| 12 | dotted header key kept only as bytes | carried in the model as `ext` (8) | war |
| 13 | missing optional atom refused | one file; a missing document is refused | text |
| 14 | unknown bold key in evidence silently drops text | continuations verbatim, no key read (3.5); the legacy reader warns on an unknown bold key | both |

## Appendix C. References (non-normative)

- RFC 2119, RFC 8174; RFC 8785 (JSON canonicalization); RFC 9562 (UUIDv7);
  llms.txt (llmstxt.org).
- WAR SAS revisions 1.0.0–1.3.0, `docs/sas/`: rationale and history. Their
  accepted bytes and signatures are unchanged.
- SAS 1.0.0-rc.3, `docs/sas/drafts/1.0.0-rc.3/`: a candidate; its footer
  format is not adopted.
- Current tool behaviour: `docs/TYPES.md`, `docs/GRAMMAR.md`,
  `docs/COMPATIBILITY.md`, `docs/TICKETS.md`.
- Extension prefix registry: `kf`, `katana`, `liminal`, `blut`
  (federation profile); `contractor`; `x` (private).

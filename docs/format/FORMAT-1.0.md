# OpenWarrant Format 1.0

| | |
|---|---|
| Status | **Draft** (`1.0.0-draft.3`, revision 1). Directed by the owner 2026-10-11 and 2026-10-12; not yet accepted. |
| Decision record | OW-ADR-0032 |
| Licence | Text: CC-BY-4.0 (`docs/format/LICENSE`). Grammar, schemas and examples: Apache-2.0 |
| Governance | `docs/format/GOVERNANCE.md` |

OpenWarrant Format is a plain-text format for engineering work: what is
wanted, why, what constrains it, how it will be checked, and how far it
has got. People write and edit it. Agents read it in small, cited pieces
that a deterministic command selects for them, and a person can read the
same selection whole and work in it. Any number of people and agents can
track work in one repository without overwriting each other. This document is self-contained. The WAR
Software Architecture Specification (SAS) is rationale and history for it
and is not normative (Appendix E).

## 1. Scope and conformance

The key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD
NOT, RECOMMENDED, MAY and OPTIONAL are to be interpreted as in RFC 2119 and
RFC 8174 when, and only when, they appear in capitals.

Conformance classes:

- A **reader** parses documents into the model (section 8). It MUST refuse
  what this document says to refuse, with the rule id of section 13.2 and a
  `path:line`. It MUST NOT drop authored text without a finding.
- A **writer** produces documents. Every document it writes MUST be
  accepted by a conforming reader. It MUST preserve the bytes of records it
  does not change.
- A **checker** is a reader that evaluates rules and reports one result per
  rule: `PASS`, `FAIL` or `UNKNOWN`.
- A **context compiler** is a reader that produces context packets
  (section 10).
- A **tracker** reads and writes work state: journals, claims and ticks,
  and answers `next` (section 11). Any number of trackers, of any make,
  may share one repository.
- An **evaluator** fires triggers and runs checks (section 12).

`UNKNOWN` means the question could not be decided: an input was missing, a
version was newer than the tool, or an extension was not understood. A
tool MUST NOT report `UNKNOWN` as `PASS` or as `FAIL`. A summary of several
results is `PASS` only when each is `PASS`.

This document defines the **core**: everything needed to read documents,
track work on them and build context from them. Two optional profiles are
defined in its appendices: the **authority profile** (Appendix C:
acceptance, authorization, independent verification, resolution and
signatures) and the **federation profile** (Appendix D: names other
systems own). A core tool need implement neither, and MUST report a rule of
one it does not implement as not applicable, never as `PASS`.

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
  `section`, `item`, `warrant`, `ticket`, `obligation`, `milestone`,
  `stage`, `check`, `kpi`, the specification types of 4.5 (`spec`,
  `requirement`, `constraint`, `outcome`, `decision`, `option`,
  `consequence`, `interface`, `term`) and `document`; other types come
  from the document's profile. A dotted type
  (`com.example.risk`) is an extension type (7.2).
- **Profile**: data (`oh.war/profile/v1`, TOML) naming the record types,
  fields, relations, sections, frontmatter keys, states and context presets
  a document may or must have (7.4 to 7.6).
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
  are U+00B7 MIDDLE DOT and U+2014 EM DASH, each between single spaces. So
  that a person can type one on any keyboard, ` | ` MAY stand for ` · ` and
  ` -- ` for ` — `: `## REQ-001 | requirement -- Title` is the same
  record. The spelling changes the unit's `revision` (its bytes) and
  nothing else: the model and the `content_digest` are those of the Unicode
  form. A heading uses one spelling for both separators; mixing them is
  refused (`format.heading`). A writer SHOULD keep the spelling it found.
  The title is REQUIRED, one line, at most 120 bytes; an empty title or a
  longer one is refused (`format.title-missing`).
  A `## ` line containing ` · `, or beginning `## <id> | `, that does not
  match MUST be refused (`format.heading`); it is never prose.
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

A line is a field when it begins `<name>:`. Otherwise it is a relation
line when it is a lowercase word or a dotted name, one space, and text;
its kind is then checked (3.6), so `blocks REQ-1` is refused
`format.relation-kind`. Any other line is refused (`format.attribute`).
A field name appears once per record (`format.field-duplicate`). A field's value is the text after
`: `, then each continuation with its first two spaces removed, joined by
LF; if the first line is empty, the value starts at the first
continuation. A REQUIRED field whose value is empty is missing; an
OPTIONAL field whose value is empty is absent. A reader
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
item, optionally ` — done by <actor>, <YYYY-MM-DD>`, a tick level ` [observed]`,
` [independent]` or ` [signed]` (11.4), and `: <note>`.
`after` names an item, a ticket, or another ticket's item. `implements
<id>` in the text is a relation. `[]`, `[y]` and other boxes are refused
(`format.item`). A box line with no id group is reported
(`format.item-id-missing`); a writer assigns an id. A group that is not at
the end of the line is no id group; a group whose id is malformed
(`(i-7)`) is refused (`format.item`). Items are read only in work
documents; elsewhere a box line is text. Item ids are unique in their document
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

### 4.5 Specifications

A repository's specification says what the system is and must do. It is
`SPEC.md` at the repository root, with profile `spec`, and MAY be split
into parts: documents with profile `spec` and `part_of: doc:SPEC.md` in
their frontmatter, which SHOULD live at `spec/<area>.md`. A repository has
at most one root specification: a file with profile `spec` that is not
`SPEC.md` and names no `part_of` is refused (`format.document-duplicate`),
whether or not `SPEC.md` exists.

```markdown
---
format: openwarrant/1.0
profile: spec
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e01
version: 0.3.0
summary: Password reset for the account service.
---
# Account service

## Purpose
Users who forget a password regain access without support staff.

## Scope
In: reset by email link. Out: SMS, account recovery without email.

## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once and only within 30
  minutes of issue.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.

## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
```

- **Identity.** The specification's own record is `doc:SPEC.md` (a part's,
  `doc:<path>`), type `spec`. Its records are global (section 5), so work
  cites them directly: `implements REQ-reset-1`.
- **Keys.** `version` is REQUIRED on `SPEC.md`: a SemVer label the author
  gives the specification as a whole. A part needs none; one it carries is
  ignored.
- **Sections.** `SPEC.md` requires Purpose and Scope (what is out of scope
  belongs in Scope). Glossary and Context are OPTIONAL. Parts require none.
  These rules for the root are core rules of this section, not profile
  data, because one profile governs the root and its parts.
- **Record types.** `requirement`, `constraint`, `outcome`, `decision`,
  `option`, `consequence`, `interface` and `term`. A `requirement` or
  `constraint` has a REQUIRED `statement`: the binding sentence, using the
  key words of section 1. `rationale` (why), `verify` (how it would be
  checked) and `priority` (`must`, `should`, `may`) are OPTIONAL. A
  `decision` relates to the options it rejected (`selected_over`) and the
  consequences it causes (`causes`); a `term`'s title is the term and its
  body the definition.
- **Context engineering.** A specification is read by agents in pieces,
  so each record carries its own meaning. A `statement` SHOULD be one
  sentence of at most 300 bytes. A checker warns
  (`context.statement-shape`, at the record's heading line) when it is
  longer, or when a `.`, `!` or `?` followed by a space occurs before its
  end. Rationale goes in `rationale`, never in the statement, so a context
  compiler can leave it out at a tight budget. A record SHOULD NOT cite
  another by section number; a checker warns (`context.section-ref`, at
  the line) on a `§` in any record of a specification. It cites the id.
- **Acceptance** is an act of the authority profile (Appendix C) and is
  held **per record**: every unit of `SPEC.md` and its parts, sections
  included (the documents' own `doc:` records are not counted). One
  `accept` covers any set of those records, usually all of them the first time and the changed ones after.
  Editing a record moves its own revision only, so only that record stops
  being accepted; every other record stays accepted, and a tool can name
  exactly which work and obligations cite the unaccepted one. A
  specification is `accepted` when every record is, `partially_accepted`
  when some are, and has no acceptance state without the authority
  profile.

A specification written before this format (`docs/sas/`) is a legacy
encoding (section 14).

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
  `doc:<path>` (a record document or specification, 4.4 and 4.5, or a
  Markdown file no profile reads). The shape routes the id.
- A work document's `uuid` is a UUIDv7 and never changes. A reference that
  must survive renaming SHOULD use `war://<uuid>`. A relation's `to` is the
  target as authored, a local id made global; a `war://` target is never
  rewritten to an alias, in the model or in a digest.
- Legacy stores (section 14) keep their ids unrenamed, including shapes
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
| no `format` | reads it as a legacy encoding (section 14) or refuses it (`format.unrecognised`). A document whose first line is not `---` declares no format |

JSON schemas follow the same rule: `oh.war/<name>/v<major>`, additive
within a major.

## 7. Extensions and unknown names

### 7.1 Names

A **dotted name** (`x.note`, `kf.enterprise_id`, `com.example.risk`) is an
extension name. The prefix `x` is private. Other prefixes MUST be a
reverse domain name or a prefix in the registry (Appendix D). An undotted
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

A **profile** says what a kind of document is: which record types,
fields, sections, relations, states and context presets it has, and which
capabilities (7.6) apply to it. Every document names one (`profile:`).
A profile MAY narrow the core: require a field, a section or a relation;
close a set. It MUST NOT loosen a core rule, change the meaning of a core
type, kind, key or state, or give an extension kind core meaning. A
document whose profile a reader cannot find is read under the core alone:
every profile rule over it is `UNKNOWN` (`format.profile-unknown`), and an
undotted name it uses that the core does not define is not refused, since
the missing profile might define it.

### 7.5 Profile data

A profile is a TOML file, `profiles/<name>.toml`, schema
`oh.war/profile/v1` (`schemas/format/1.0/profile.json`), that declares
`format = "openwarrant/1.0"`; a profile file without that key is a legacy
profile (section 14) and never governs a Format 1.0 document. The built-in
profiles of 7.6 need no file. A repository file with a built-in's name
replaces it only by naming it in `extends` (`extends = "ticket"` in
`profiles/ticket.toml` starts from the built-in); without that it is
`profile.invalid`. Keys:

| key | value |
|---|---|
| `schema` | `"oh.war/profile/v1"` |
| `format` | `"openwarrant/1.0"` |
| `name` | the profile's name; equals the file stem |
| `extends` | OPTIONAL: a profile whose declarations this one starts from. It may add and narrow, never remove: capabilities are the union, a field either requires is required, and its `form` is the base's |
| `form` | `work` (its documents are work documents, 4.1) or `document` (4.4, 4.5) |
| `note` | one line saying what the profile is for |
| `capabilities` | a subset of the closed set of 7.6 |
| `[frontmatter]` | `required`, `optional`: undotted keys beyond those of 3.3 |
| `[sections]` | `required`, `optional`: section names; a document with a section neither lists is accepted unless `closed = true` |
| `[records]` | `types`: the record types its documents may hold beyond the core's; `closed = true` refuses any other undotted type (`format.key-unknown`) |
| `[fields.<type>]` | per record type: `<name> = { required = bool, shape = <shape>, values = [...] }` |
| `[relations]` | `allow` (core kinds its records may use) and `require` (`[[from-type, kind, to-type]]`) |
| `[[states]]` | declared states: `name`, `refines` (a fixed state of 11.1), `note` |
| `[ticks]` | the minimum tick level (11.4): `item`, `milestone`, and `[ticks.types]` by the document's `type` key |
| `[[presets]]` | context presets (10.4): `name`, `steps`, `depth`, `budget` (bytes or `"none"`), `omit_types`, `omit_fields`, `editable`, `reads_state` |

**Shapes.** `text`: anything, continuations included. `line`: one line.
`int`: an optional `-` and decimal digits. `number`: an `int`, optionally
followed by `.` and digits. `date`: `YYYY-MM-DD`. `id`: one target (3.6).
`ids`: targets separated by `, `. `gate`: `gate://<id>@<version>`, several
separated by `, `. `enum`: one of `values`. `bool`: `true` or `false`.

A field a profile declares but a record lacks is refused when `required`
(`format.field-missing`). A value that does not have its `shape` is
refused (`format.field-shape`). Once the core or the profile declares a
field table for a type, a field outside it is refused
(`format.key-unknown`), with 7.2's exception for a newer minor; a type for
which neither declares any field accepts any field. A section outside a
`closed = true` set is refused the same way. A profile file is refused
(`profile.invalid`), and its documents read as `format.profile-unknown`,
when it has an unknown key; a `name` that is not its file stem; a
capability outside 7.6 or without its prerequisite; a `refines` naming no
fixed state of 11.1; a preset named like a built-in; an unknown shape, or
an `enum` without `values`; an `extends` naming no profile, or a `form`
other than its base's.

A **pack** is a versioned set of profiles (`oh.war/pack/v1`: `name`,
`version`, `profiles`) installed into `profiles/` and recorded in
`docs/types.toml` with its digest, so a repository can say which
definition of a type it uses.

### 7.6 Built-in profiles and capabilities

**Capabilities**, a closed set. Each turns on a family of rules and
states; a rule whose capability a profile lacks is not applicable and is
reported as such, never as `PASS`.

| capability | turns on |
|---|---|
| `structure` | sections, records and fields (3, 4) |
| `links` | relations and their targets (3.6), `superseded` |
| `claims` | items, claims, `open`/`in_progress`/`done` (11) |
| `checks` | `check` and `kpi` records and triggers (12) |
| `stages` | milestones and stages (4.1) |
| `acceptance` | `accepted` (Appendix C) |
| `evidence` | obligations' evidence and gate runs |
| `verification` | independent verdicts and dispositions (Appendix C) |
| `authorization`, `resolution` | the acts of Appendix C |

A capability's prerequisites: every other capability needs `structure`;
`verification` needs `evidence`; `resolution` needs `authorization`;
`checks` needs `claims`. These govern Format 1.0 profiles; legacy
profiles keep their own (section 14).

**Built-in profiles.** Each is defined by a profile file shipped with this
document (`schemas/format/1.0/profiles/`), which is normative.

| profile | form | capabilities | requires |
|---|---|---|---|
| `ticket` | work | structure, links, claims, checks | Checklist |
| `delivery` | work | all | Intent, Basis, Work order, Assurance; at least one obligation (a core rule of 4.1, which profile data cannot state) |
| `decision` | work | all but `stages` | Intent, Basis |
| `spec` | document | structure, links, acceptance | 4.5 |
| `adr` | document | structure, links, acceptance | Context, Decision, Consequences |
| `roadmap` | document | structure, links, claims | nothing; its phases are `phase` records |
| `release`, `incident`, `prd`, `architecture`, `test-plan` | document | structure, links | nothing; their files list the sections they recommend |

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
  `content_digest` in every encoding (section 14).

A new signature SHOULD bind both. A digest computed under 1.0 MUST
recompute identically under every later 1.x.

## 10. Context

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

`<repository name>` is the name of the repository's root directory.
Sources are sorted by path, each group preceded by one blank line, and
records by line. A work document is listed by its own record alone; its
units are one `war context <alias>` away. Every record of any other
document is listed. The `> ` line is the document's `summary`, or `-`.
`<state>` is the record's **primary state** (11.1), or `-`. A record with no title shows its first non-blank
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
walked, and the containment `part_of` of 4.1, 4.4 and 4.5 is walked like
an authored one, so a requirement reaches its specification. A record's **distance** is the level that first reached it, and
its **via** the first `(step, from)` that did, by step order and then by
`from` id. Unknown targets are skipped with a diagnostic. This walk is
named `core`. A profile MAY declare another walk; the packet names the
walk it used.

**3. Order.** By distance, then type rank — `warrant`/`ticket`, `section`,
`item`, `requirement`, `outcome`, `constraint`, `decision`, `interface`,
`term`, `option`, `consequence`, `obligation`, `check`, `kpi`,
`milestone`, `stage`, `instruction`, any other — then source path, then line.

**4. Render.** Markdown, LF line endings:

```text
# Context: <target>
<!-- oh.war/context/v1 · budget <B> · depth <d> · preset <preset> (walk <walk>) · selection <selection_digest> -->

## Index
[F] <id> · <type> · <state> · <title> · <source>:<line>
[S] <id> · <type> · <state> · <title> · <source>:<line>

## Records
> <id> · <type> · <source>:<line> · sha256:<first 12 hex of revision>
<the unit's exact span, with a final LF added if missing>

```

The header always names the preset (10.4), `agent` when none was asked
for. `<B>` is `none` for a preset without a budget. A document's own
record has as its span the frontmatter and the preamble, up to its first
unit; that span gives its text and revision.

`selection_digest` is the `basis_digest` rule (section 8) over the
selected records. In the index, `[F]` marks a record whose full text
follows and `[S]` one given only by its summary. `<state>` is the primary
state of 11.1, or `-`. An item whose unit (the section or record whose
span holds its line) is given in full is marked `[F]` and has no entry of
its own under Records: its line is already there. An item that is the target has its own entry.

**5. Budget.** Bytes are UTF-8 bytes of the Markdown rendering. If the
header and index alone, or with the full text of every root, exceed the
budget, the compiler MUST refuse (`context.over-budget`), naming the bytes
needed, and emit nothing. Otherwise records are given in full in order
until the next one would not fit; it and every later record are `[S]`. A
record is never cut. An `[S]` line is the agent's cue to run `war context
<id>`. A selected record whose text cannot be read from its source (a
legacy record whose span the model does not give) is given by its summary
only, with a warning (`context.span-unknown`).

`--json` emits `oh.war/context/v1`, the same selection: `{schema, target,
budget, bytes, depth, walk, selection_digest, records[{id, type, title,
state, source, line, revision, distance, via?{step, from}, tier, text?}],
diagnostics[]}`, `tier` being `full` or `summary` and `text` the exact
span for `full` (an item's line). `title` is always present, `-` when the
record has none; `diagnostics` have the model's shape. `bytes` is the size
of the Markdown rendering. A tool MAY wrap the packet in its own report
envelope; the packet is unchanged inside it.

### 10.4 Presets

A **preset** is a named set of choices for the context compiler, so one
compiler serves an agent that needs the least that is sufficient and a
person who needs everything the document type holds.

`war context <target> --preset <name> [--at <time>]`

A preset is profile data (7.5) and declares: the walk and its steps; the
depth; the budget, or `none`; the record types and field names it leaves
out; whether its records are **editable** (10.5); and whether it **reads
state**. Without `--preset` the preset is `agent`. A preset that walks
the core steps names its walk `core`; one with steps of its own names its
walk after itself.

| preset | for | selection |
|---|---|---|
| `agent` | an agent doing the work | 10.3 exactly: core walk, depth 3, budget 16000 |
| `agent.min` | a tight window | depth 1 over `out:implements`, `in:constrains` and `in:evaluates`; leaves out `option` and `consequence` records, and cuts each `rationale` field (its line and continuations) from the text it gives, the `>` line still naming the source revision; budget 4000 |
| `human` | a person reading and working | the core walk, depth 3, no budget, every record `[F]`, editable; the work document's own units first in document order, then the rest in 10.3's order |
| `review` | a reviewer or verifier | depth 2 over `in:evaluates` (obligations and checks of the target) then `out:evaluates` (what they evaluate), no budget; reads state |
| `resume` | whoever picks the work up | the target's record and items, the core walk to depth 1, no budget; reads state; rendered as below |

**`resume` rendering.** After each item's `>` line comes one state line:
`state <primary>[ · holder <actor> · lease <until>][ · by <actor> ·
<date> · <level>]`, the holder part for a claimed item and the `by` part
for a done one. After Records, a section `## Next` lists up to three of
the target's candidates in the order of 11.5, one `- <id> · <title>` per
line.

A preset that reads state also reads the journals and claims of section
11 and is a pure function of the documents, those files and the time
given by `--at` (a tool MAY default it to its clock). Its header records
both: `· journal sha256:<12 hex> · at <time>`, the digest being the
`basis_digest` rule (section 8) over `[path, sha256 of the file's bytes]`
for every journal and every claim file, sorted by path. Every other preset is bound
by 10.3's purity rule. A profile MAY declare further presets; it MUST NOT
redefine a built-in one.

### 10.5 Working in a package

An editable preset wraps each record's text in markers that name its exact
source:

```text
<!-- rec OW-WAR-0146/S-intent docs/warrants/OW-WAR-0146.md:7 sha256:<64 hex> -->
## Intent
…
<!-- /rec -->
```

The marker names the repository-relative path and line. The `>` line of
10.3 is kept above the marker. A document's own record (its frontmatter
and preamble) is not wrapped and cannot be edited through a package.
A person MAY edit the text between markers and give the package back:
`war apply <package>`. For each marked block, a writer:

1. ignores it when its text equals the source span at the marked revision;
2. otherwise refuses it (`apply.stale`) when the source span's current
   revision is not the marked one: someone changed it since the package
   was made;
3. otherwise parses the block as a unit under section 3 and refuses it
   (the rule id of the fault) when it does not parse, or when its id or
   type changed (`apply.identity`);
4. otherwise replaces the span's bytes with the block's.

A changed item box is not a text edit: `[ ]` to `[x]` records `done`, and
`[x]` to `[ ]` records `reopen`, as events of section 11 by the actor
applying the package, and the writer then rewrites the line as 3.8 shows,
dated with the date of `war apply`'s time (`--at`, default its clock).
A new box line without an id is given one. Blocks of one document are
applied together or not at all; a refusal in one document does not stop
another. A block missing from the package is not a deletion; it is left
alone. Text outside markers (the index, headers, state lines) is never
written back. Every applied change goes through the compare-and-set of
11.3.

## 11. State and work tracking

Work is tracked so that any number of people and agents on one repository
always know what is next and never overwrite each other. Documents say
what the work is; the journal and the claim store say what has happened
to it. Neither is authored by hand.

### 11.1 States

A state is computed (section 2) and appears in the model (`states[]`).
Each has a `kind`, the family it belongs to, and a `provenance`:
`computed` (from documents and the claim store), `recorded` (from a journal
event or an act of Appendix C) or `declared` (below). The **fixed
states**, which no profile may redefine:

| kind | state | of | holds when | capability |
|---|---|---|---|---|
| `phase` | `open` | work document | not `done`, and no item is done or claimed | claims |
| `phase` | `in_progress` | work document | an item is done or claimed, or the document itself is claimed, and it is not `done` | claims |
| `phase` | `done` | work document | it has items and every item is done; with no items, a `work.item_done` names the document | claims |
| `phase` | `draft`, `authorized`, `resolved` | work document | Appendix C: under `authorization`, a work document is `draft` until authorized | authorization |
| `checklist` | `open`, `claimed`, `blocked`, `done` | item | `done`: its box is `[x]`; `claimed`: a live claim names it (11.3); `blocked`: an item or document it waits on (`after`, `depends_on`, or its document's `depends_on`) is not done; else `open` | claims |
| `currency` | `superseded` | any record | another current record `supersedes` it | links |
| `check` | `failing` | check, kpi | its last recorded run on its current revision failed (12) | checks |
| `disposition` | the disposition | obligation | Appendix C | verification |
| `acceptance` | `accepted`, `partially_accepted` | specification and its records | Appendix C | acceptance |
| `outcome` | `verified` | work document | Appendix C | verification |

A work document with no items is workable as it is: it is `open` until it
is claimed or done, and `claim` and `done` act on the document itself. A
work document `part_of` another counts as an item of it, for the parent's
phase and for `next`. An item whose box is `[x]` is done, whether a tool
or a person ticked it; how it was done is its tick level (11.4).

A **declared state** (7.5) refines one fixed state: it is entered by a
`state.entered` event naming it, holds only while its parent holds, and
lapses when the parent stops holding; the later of two entries wins. A
declared state satisfies no rule and no check. Its `kind` is its parent's
and its provenance `declared`.

The **primary state** shown in the index and in packets is the first that
holds of: `superseded`; the record's `phase`; its `checklist` state; its
`check` state; its `disposition`; its `acceptance`; else `-`. A declared
state that holds is shown after its parent: `in_progress/in_review`.

### 11.2 The journal

Each work document has one journal, `journal.jsonl`, in its evidence
directory (4.3): `docs/warrants/<alias>/journal.jsonl`,
`docs/tickets/<alias>/journal.jsonl`. Each line is one event, RFC 8785
JSON:

```text
event = { v: 2, id, document_uuid, type, class, actor_ref, occurred_at, payload, idempotency_key }
```

| member | value |
|---|---|
| `v` | `2` |
| `id` | a UUIDv7 |
| `document_uuid` | the work document's `uuid` |
| `type` | the event type, below |
| `class` | `draft_history`, `action_request` or `action_receipt` |
| `actor_ref` | who: a name for a person or agent, `person://<name>` under Appendix C |
| `occurred_at` | RFC 3339, UTC |
| `payload` | the event's payload, a JSON object |
| `idempotency_key` | lowercase hex sha256 of `<document_uuid> LF <type> LF <RFC 8785 bytes of payload>` |

A tracker writes `v: 2` events into the journal of a Format 1.0 work
document. Journals written before this format hold `v: 1` events, which
name the document `warrant_uuid` and carry `payload` as RFC 8785 JSON text
(a string), with the key over that text. A reader MUST read both, forever,
and MUST NOT rewrite a `v: 1` line; one journal may hold both. A tracker
appending to a legacy directory work document keeps writing `v: 1`, so
that document's journal has one shape.

- **Append-only.** A journal only grows: a tool MUST refuse a journal whose
  committed lines (those of the version in the current commit, `HEAD`)
  are not all still present, unchanged and in the same order
  (`work.journal-rewritten`); lines added between them, as a union merge
  adds them, are not a change. It MUST refuse a line that does not parse
  as JSON, lacks a member, has a `v` other than 1 or 2, or whose
  `idempotency_key` does not match its content (`work.journal-malformed`),
  and an unterminated last line (`work.journal-torn`). A line with `v`
  greater than 2 is `UNKNOWN` (`work.journal-newer`), not malformed, and
  every state that depends on it is `UNKNOWN`. A line's bytes need not be
  RFC 8785; a reader parses it. A repository MUST mark journals
  `merge=union` (`**/journal.jsonl merge=union` in `.gitattributes`), so
  two branches that both appended merge to both events.
- **Exactly once.** An event whose `idempotency_key` is already in the
  journal from the same actor is a replay and is not written; from another
  actor it is refused (`work.idempotency-conflict`). Time is not in the
  key, so ten runners recording the same fact record it once.
- **No clock in meaning.** No state depends on the order of lines. Where
  two events about the same target conflict (`work.item_done` and
  `work.item_reopened`, two `state.entered`), the one with the later
  `occurred_at` wins, and of two with the same time the one with the
  greater `id`.

Core event types and their payloads (other types are extension types,
dotted, and carried). In payloads, `target` is the global id of what the
event is about (`t-0d61d/i-4d11`, or an alias for a whole document) and
`item` the item's local id:

| type | payload |
|---|---|
| `work.created` | `{document, title, items[]}` |
| `work.item_added` | `{item, text, after?}` |
| `work.claimed` | `{claim, target, since, lease_until?}` |
| `work.claim_reclaimed`, `work.claim_stolen` | as `work.claimed`, with `from`, `from_since` |
| `work.released` | `{claim, target}` |
| `work.item_done` | `{item, target, on, level?, runs[]?, note?}` |
| `work.item_reopened` | `{item, target, reason}` |
| `work.note_added` | `{target, note}` |
| `work.edited` | `{target, from_revision, to_revision}` |
| `work.tick_raised` | `{item, target, from, to, verifier? , signer?}` |
| `check.run` | `{check, run, verdict, revision, subjects, commit?, output_sha256?, value?}` (12): `verdict` is `PASS`, `FAIL` or `UNKNOWN`; `revision` the check's; `subjects` the `[id, revision]` of each record or item it evaluates, at the run |
| `trigger.fired` | `{check, trigger, subject}` (12) |
| `state.entered` | `{record, state, refines, note?}` |

Journals written before this format use `ticket.*` names for the same
events (`ticket.claimed`, `ticket.item_done`…); a reader MUST read them as
the `work.*` event of the same suffix.

### 11.3 Claims and writes

**Claims.** Before working on an item (or a work document with no items)
an actor claims it. A claim is a file in the **claim store**, shared by
every working copy of a clone: `<git common dir>/openwarrant/claims/`, or
`.openwarrant/state/claims/` outside version control. The file
`<alias>.lock` or `<alias>--<item>.lock` holds `oh.war/ticket-claim/v1`
JSON `{schema, claim, ticket, item?, actor, since, since_unix,
lease_until?, lease_until_unix?}` and is created atomically (create a
temporary file, then hard-link it to the name; the link fails if the name
exists). The store is never committed.

- **Exclusive.** A second claim on a claimed target is refused
  (`work.claimed-by-other`), naming the holder. A claim on a work document
  covers all its items, so it is refused while another actor holds one of
  them, and a claim on an item is refused while another actor holds its
  document.
- **Leases.** A claim's lease runs `claim_lease_minutes` (default 30) past
  the later of `lease_until` and the file's modification time; the holder
  renews it by touching the file (`war heartbeat`, or any command the
  holder runs). A claim whose lease has run out is reclaimable by a plain
  claim, recorded as `work.claim_reclaimed`. A live claim whose `since`
  is more than `claim_ttl_minutes` (default 120) ago may be taken only on
  request (`--steal`), recorded as `work.claim_stolen`. Both settings live
  in `openwarrant.toml` under `[claims]`.
- **Across machines.** With `[claims] remote = "<name>"`, a claim is also
  the ref `refs/openwarrant/claims/<alias>[--<item>]` on that remote,
  pushed with `--force-with-lease` against the value read, so the remote
  decides which of two machines won; the loser's local claim is removed and
  it is refused (`work.claimed-by-other`). A remote that cannot be reached
  is `UNKNOWN` (`work.claim-remote-unreachable`), never a granted claim.
- `done` and `release` end the claim.

**Compare-and-set.** Every write to a work document names the revision it
was based on: the item's line revision for a box, the unit's revision for
a unit, the document's for a frontmatter change. A write whose base is not
the current revision is refused (`work.stale-revision`), naming the
current one, and writes nothing. A writer takes an exclusive lock on the
document's evidence directory for the length of a write. Together these
mean no write is ever lost: a writer that lost a race re-reads and tries
again, or reports the refusal.

### 11.4 Ticks and their level

A done item records how its completion was established, its **level**,
from weakest to strongest: `claimed` (someone said so), `observed` (a
check ran and passed, 12), `independent` (a verifier who is not the
performer agreed, Appendix C), `signed` (a person signed it off, Appendix
C). The level is written in the done suffix (3.8): `— done by claude,
2026-10-12 [observed]: note`; `claimed` writes none. A box ticked by
hand, with no event, is done at `claimed`. A level above `claimed` is
believed only as far as the journal backs it: `observed` needs a
`work.item_done` with `level: observed` and passing `runs`, or a
`work.tick_raised` to `observed`; `independent` and `signed` need the act
of Appendix C. An unbacked level reads down to the highest backed one
(`work.tick-unbacked`, a warning). A display MUST NOT show a `claimed`
tick as verified.

A profile's `[ticks]` sets the minimum level an item or milestone needs;
the repository's preset (Appendix C) MAY raise it. An item done below its
minimum is `done` for `next` and is listed as below its minimum.

### 11.5 What is next

`next` answers, for an actor, what to work on, as a pure function of the
documents, journals, claim store and a time. Its **candidates** are the
items whose state is `open` (so neither blocked, claimed under a live
claim, nor done; an expired claim does not count), and each work document
with no items whose phase is `open`. A work document `part_of` another
counts as an item of it. An item has its document's `due`, `priority`,
`created_at` and labels. Candidates are ordered by:

1. `due`, earliest first; a candidate inherits the earliest `due` of
   everything that waits on it; none sorts last;
2. remaining path, longest first: the candidate's estimate plus the
   longest remaining path of what waits on it (the critical path first);
3. `priority`, lowest number first (default 2);
4. `created_at`, earliest first, none last; then alias; then the item's
   position in its document.

An **estimate** is the median, over done items, of the time from an item's
last `work.claimed` to its `work.item_done`, taken over items of the same
`type` sharing a label, else sharing a label, else of the same `type`,
else all, in whole seconds (with an even count, the lower of the two
middle values); with no samples it is `prior_secs` (default 1800, in
`openwarrant.toml` under `[next]`). Ten actors
asking at once get the same order; each then claims the first candidate it
can, and 11.3 makes exactly one claim win. After the candidates, `next`
lists acts that wait on a person (Appendix C).

## 12. Checks and triggers

Work can finish itself. A **check** is a record saying what to run to
establish something and when to run it; when it passes, the items it
evaluates are ticked `observed`, with the run as evidence.

```markdown
## CHK-reset · check — reset tests pass
run: cargo test -p accounts reset::
on: done, pr.merged
evaluates i-9c01, i-4d11
timeout: 600
```

| field | | value |
|---|---|---|
| `run` | REQUIRED | a command, run by `sh -c` in the repository root, or `gate://<id>@<version>` |
| `on` | OPTIONAL | triggers, comma-separated (below); default `done` |
| `timeout` | OPTIONAL | seconds; default 600 |
| `gating` | OPTIONAL | `true` (default): a failure blocks the tick; `false`: recorded only |
| `reopen` | OPTIONAL | `true`: when the check turns `failing`, a done item it evaluates is reopened (`work.item_reopened`); `false` (default) |

A **kpi** is a check that measures: its command prints a number, and it
has a REQUIRED `direction` (`min` or `max`), an OPTIONAL `target` (shape
`number`), and `mode`:
`threshold` (passes when it meets the target), `best` (passes when it
meets the target and records the best value seen, default) or `optimise`
(never fails; records the value).

**Triggers.** When a check runs:

| trigger | fires when |
|---|---|
| `manual` | someone asks (`war check run <id>`) |
| `done` | a tracker marks an evaluated item done; the item is ticked only if the run passes. A box ticked by hand fires nothing: it is done at `claimed`, and an evaluator that later runs the check and sees it pass raises it to `observed` |
| `commit` | a commit changes the document or a path in its `paths` |
| `paths:<glob>` | a commit changes a matching path, so work already done is checked again |
| `pr.merged` | a pull request citing the work merges |
| `ci:<name>` | the CI check `<name>` reports on a commit |
| `schedule:<cron>` | at the times a five-field cron expression names, in UTC |

A dotted trigger (`x.deploy`) is an extension: carried, never fired by a
tool that does not understand it.

**Semantics.**

- A trigger is fired by an **evaluator**: `war` itself (`war done`, `war
  watch`), a version-control hook, or a CI step. A document only declares
  triggers. Firing records `trigger.fired`; the run records `check.run`
  with its verdict and the sha256 of its output, kept beside the document
  (4.3). Both events' idempotency keys (11.2) contain the check's revision
  and the subject (commit or time), so several evaluators firing the same
  trigger record it once.
- A **pass** on the check's current revision ticks every open item it
  evaluates at `observed` (11.4) and raises a done item it evaluates to
  `observed` when it was below, through the compare-and-set of 11.3, and
  records an observation for every obligation it evaluates. A check never
  sets a disposition; that is Appendix C's verdict.
- A **fail** ticks nothing and makes the check `failing`; with `reopen:
  true` it also reopens each done item it evaluates. A run that could
  not complete (timeout, missing tool, no number from a kpi) is `UNKNOWN`
  (`check.unknown`): it ticks nothing and is never reported as a pass or a
  fail.
- A run counts only for the revisions it names: a pass whose check
  `revision`, or whose `subjects` revision for an item or record, is not
  the current one ticks nothing for it. `failing` looks only at runs on the
  check's current revision, the last by `occurred_at`.

## 13. Conformance

### 13.1 The corpus

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
  journal/**, claims/**, at     # tracking cases: journals, claim store, the time
  expected/next.json   # tracking cases: the candidates of 11.5, in order
  expected/states.json # tracking cases: the states[] of the model
  package.md           # apply cases: the edited package
  expected/after/**    # apply cases: the documents after `apply`
```

Exactly one `expected/` result is present. Tracking and apply cases are
derived from sections 10.5, 11 and 12 as the others are from 3 to 10.
How a case names its operation, gives a committed journal, a claim
file's modification time or an act without a signature, and spells a
tracking refusal, is fixed by `conformance/format/README.md`, which is
part of the corpus. A conforming tool passes a case when its output equals the
expected bytes. `model.json` holds the compared subset of the model that
`conformance/format/README.md` defines, after RFC 8785 canonicalization,
its `diagnostics` compared as a set of `(rule, record)`. A context refusal
is one line, `<rule id>`. Every refusal rule of 13.2 has at least one
case, and every claim has a case that is refused.

### 13.2 Rule ids

Refusals: `format.encoding`, `format.frontmatter`, `format.key-unknown`,
`format.heading`, `format.title-missing`, `format.attribute`,
`format.field-duplicate`, `format.field-missing`, `format.authored-state`,
`format.relation-kind`, `format.relation-target`, `format.item`,
`format.id-duplicate`, `format.fence-unclosed`, `format.section-missing`,
`format.document-duplicate`, `format.manifest-restates`,
`format.unrecognised`, `format.field-shape`, `profile.invalid`,
`context.target-unknown`, `context.over-budget`, `apply.stale`,
`apply.identity`, `work.stale-revision`, `work.claimed-by-other`,
`work.journal-rewritten`, `work.journal-malformed`, `work.journal-torn`,
`work.idempotency-conflict`.
Unknown: `format.newer-major`, `format.newer-minor`,
`format.requires-unknown`, `format.profile-unknown`,
`work.claim-remote-unreachable`, `work.journal-newer`, `check.unknown`. Warnings:
`format.target-unknown`, `format.refusal-missing`, `format.item-id-missing`,
`format.legacy-obligation`, `context.summary-shape`,
`context.statement-shape`, `context.section-ref`, `context.span-unknown`,
`work.tick-unbacked`, `format.prefix-unregistered`.

A finding about a whole document (a missing fence, key or section, a
restated manifest key) is reported at line 1; `format.document-duplicate`
at `<alias>.md:1`. A reader reports each fault once: a unit it refused is
not also reported missing.

## 14. Legacy encodings (non-normative)

Stores written before this format are read forever, byte for byte. Nothing
is migrated and no signed digest moves. Each lowers to the same model; a
legacy unit keeps the revision of its legacy bytes and has the
`content_digest` of its Format 1.0 equivalent.

| encoding | lowers to |
|---|---|
| `oh.war/records/v1` record atom | records without titles; spans run to the next record heading, as before; relation lines anywhere in a record |
| directory Warrant: `manifest.toml`, `atoms/*.md`, `45-milestones.yaml`, `deliverables.toml` | `uuid`, `local_alias` (→ `alias`), `title`, `profile` and, when present, `assurance_level` → document fields; intent, basis and work order atoms → `S-intent`, `S-basis`, `S-work-order`, any other atom role → `S-<role>` (`S-<role>-2` on a repeat); milestones (ids kept, `<alias>/M1`) and stages → records; each `### OBL-NNN — statement` block → an obligation, its `- **key:**` bullets → fields; each deliverable → a `deliverable` record with its `[[…]]` span, a title and a `target_ref` field, and the deliverables' `target_ref`s are the document's `paths`; `[[implements]]` → `implements`; `[[supersedes]]` → `supersedes`; `[[parents]]` → `parent`; `[[roadmap]]` → `roadmap`; `enterprise_id` → `kf.enterprise_id`, omitted when empty |
| ticket, `oh.war/ticket/v1` | a work document, profile `ticket`; manifest keys → document fields; `promoted_to` → a relation; `15-checklist.md` → `S-checklist`, lines unchanged |
| SAS document and revisions, `docs/sas/` | a `spec` record; sections and requirement rows → records with their ids (`WAR-SAS-101.1`). Its accepted revisions keep their acceptance, under the document digest they were signed over |
| ADR atom | one record, id the alias, type `adr`; `adr_uuid` → `uuid`; `status` → field; `governs`, `supersedes` → relations |
| roadmap, `docs/roadmap/` | a `roadmap` record; each phase a `phase` record `part_of` it, with `depends_on` |

**Migrating a specification.** A writer MAY migrate a legacy SAS into a
specification (4.5) without changing it: it writes `SPEC.md` (Purpose and
Scope from the SAS's opening sections) and one `spec/<area>.md` per
top-level SAS section, each numbered section a section of its part, each
§106 requirement row a `requirement` with id `REQ-` and the row's number
with `.` replaced by `-` (`WAR-SAS-101.1` → `REQ-101-1`), its text as
`statement`, and the old id as the field `legacy.id`. The SAS files stay
where they are and stay accepted; the new specification is not accepted
until someone accepts it (Appendix C). A reader resolves an old id to the
new record through `legacy.id`, so existing citations and pins keep
working.

Legacy kinds and ids are carried unrenamed, so existing output and
signatures stand. A legacy work document has no `summary`; its document
record has the `content_digest` of a Format 1.0 equivalent with the same
keys, and every other unit has its equivalent's.

## 15. Changes from current practice

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
13. The SAS is rationale. Records cite ids, not `§`. A repository's
    specification is `SPEC.md` and its parts (4.5).
14. Record headings may be typed in ASCII (` | `, ` -- `).
15. Profiles are normative data (7.5) and capabilities a closed set (7.6).
16. States, the journal, claims, compare-and-set, tick levels and the
    order of `next` are part of the format (11), so independent tools can
    share one repository's work.
17. Checks and triggers let work finish on observed evidence (12).
18. Context presets serve agents and people from one compiler, and a
    person's edits to a package flow back (10.4, 10.5).
19. The authority and federation profiles are written down (Appendices C
    and D).

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
section-name  = text - ( text " · " text | record-id " | " text ) ;
record        = record-head EOL attributes [ blank EOL body ] ;
record-head   = "## " record-id ( " · " type " — " | " | " type " -- " ) title ;
type          = word | dotted ;
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
done          = " — done by " actor ", " date [ " [" level "]" ] [ ": " text ] ;
level         = "observed" | "independent" | "signed" ;
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

## Appendix C. The authority profile (normative for tools that claim it)

The authority profile lets a repository require that people, not tools,
make certain decisions, and record each one so anyone can check it later.
A tool that claims this profile implements all of it.

### C.1 Who may act

`docs/authority/roles.toml` assigns roles: `[[assignment]]` with `actor`,
`actor_kind` (`human`, `agent` or `policyservice`), `roles` (from
`performer`, `verifier`, `authorizer`, `resolver`, `risk_acceptor`,
`judge`), `assigned_by`, `effective_time`, `note` and, for a person who
signs, `ssh_principal`. `docs/authority/allowed_signers` lists each
signing principal's public key in OpenSSH `allowed_signers` form, with
`namespaces="oh.war/response"`. Only a tool reading a person's answers at
a terminal writes either file, once.

### C.2 Acts

| act | made by | subject | state it gives |
|---|---|---|---|
| `accept` | authorizer | records of a specification (4.5): all of them, or those not yet accepted | `accepted`, per record |
| `authorize` | authorizer | a work document | `authorized` |
| `verify` | verifier, not the performer | an obligation | a disposition; `verified` when every required obligation is established |
| `resolve` | resolver | an authorized work document | `resolved` |
| `correct` | authorizer | a delivered file of a resolved work document | a new pinned digest |
| `sign-off` | a person | an item | its tick level `signed` |
| `approve` | a person allowed by C.4 | a work document | it is **official** (C.4) |

An act is a **response file**, `docs/authority/responses/<subject>.<act>.response.toml`
(for `accept` and `authorize`, `<subject>.response.toml`), with a detached
SSH signature beside it (`.sig`), made with `ssh-keygen -Y sign` under
namespace `oh.war/response`. It names the act's schema
(`oh.war/<act>-response/v1`), the actor, the acting role, what the act
means in a sentence, the effective time, and the **subject digest**. It is
valid when the signature verifies against `allowed_signers` for the
principal that `roles.toml` binds to the named actor, that actor is
`human`, holds the role, and the subject digest equals the subject's digest
now. A tool MUST NOT make an act itself, and MUST refuse a response whose
actor is an agent or whose verifier is the subject's performer.

### C.3 What an act binds

For a Format 1.0 subject, the **subject digest** is the digest (section 9),
domain `oh.war/act-subject/v1`, over the array of `[id, revision,
content_digest]` of every record the act covers, sorted by id: the whole
document for `authorize` and `resolve`; the specification records it
names for `accept`; the obligation and its evidence for `verify`; the item
for `sign-off`. Binding both digests means a signature holds while the
bytes are unchanged, and a reader can still tell, after a reformat, that
the meaning did not move. When a covered record's revision changes, the
state the act gave lapses. For `accept` it lapses for that record alone:
the specification becomes `partially_accepted`, a later `accept` naming
the changed records restores it, and until then `next` lists that
acceptance as an act waiting on a person and a tool reports the work and
obligations that cite the unaccepted record. For the other acts it lapses
for the whole subject. Legacy subjects keep the digests their acts were signed over
(section 14).

### C.4 Presets, roles and official work

`openwarrant.toml` `[preset]` chooses how much of this profile a
repository uses:

| preset | signing | minimum tick | official work required to merge |
|---|---|---|---|
| `vibe` | never | `claimed` | no |
| `team` | at release, one signature over a batch | `claimed` | yes |
| `regulated` | at merge | `observed` | yes |

`[roles]` maps a forge role (`admin`, `maintain`, `write`, `triage`,
`read`, `none`) to the least formal kind of work it may create alone. Work
is **official** when an allowed person made it or approved it (`approve`,
or an approving review by an allowed role). Under `team` and
`regulated`, a change may merge only when it cites official work at its
author's level. A tool that cannot establish the author's role reports
`UNKNOWN`.

### C.5 Independence

A verdict counts toward a disposition only when its verifier is not the
performer of the work it judges (the performer is read from the work's
record, never from the response), it names the evidence it saw, and the
work and evidence it names are the current revisions. A verdict that
fails any of these is recorded and counts for nothing.

## Appendix D. The federation profile (normative for tools that claim it)

Other systems may annotate documents with what they own. Their names are
extension names (section 7) under registered prefixes; none is core, and
a tool that does not implement this profile carries them and ignores them.
A federated system stores references and digests in a document, never a
copy of its own facts.

| prefix | system | keys and references |
|---|---|---|
| `kf` | Knowledge Fabric: enterprise identity, authority and lifecycle | `kf.enterprise_id` (a work document's id in the fabric) |
| `katana` | Katana: agent execution and runtime capture | `katana://` references to runs; receipts `katana/openwarrant-receipt/v1`, verdicts `katana/openwarrant-verification/v1` |
| `liminal` | Liminal: a host that compiles documents for a workspace | the exchange `oh.war/liminal-v1` (a request of documents as bytes, a response of the model and packets); basis ids `liminal-basis://<digest>` |
| `blut` | BLUT: plan lowering and backends | `blut://<backend>@<pin>`; verdicts `blut/openwarrant-verification/v1` |
| `contractor` | contractor work orders | acceptance, invoicing and legal terms outside the technical core |
| `legacy` | readers of earlier encodings (section 14) | `legacy.id`: the id a record had in a legacy store |
| `x` | private use | anything; never registered |

A prefix is registered by the editor (GOVERNANCE.md). A checker warns
(`format.prefix-unregistered`) on a dotted name whose prefix is neither
registered nor a reverse domain name, that is a name of three or more
labels whose first label is letters only (`com.example.risk`).

## Appendix E. References (non-normative)

- RFC 2119, RFC 8174; RFC 8785 (JSON canonicalization); RFC 9562 (UUIDv7);
  llms.txt (llmstxt.org).
- WAR SAS revisions 1.0.0–1.3.0, `docs/sas/`: rationale and history. Their
  accepted bytes and signatures are unchanged.
- SAS 1.0.0-rc.3, `docs/sas/drafts/1.0.0-rc.3/`: a candidate; its footer
  format is not adopted.
- Current tool behaviour: `docs/TYPES.md`, `docs/GRAMMAR.md`,
  `docs/COMPATIBILITY.md`, `docs/TICKETS.md`.
- The extension prefix registry is Appendix D.

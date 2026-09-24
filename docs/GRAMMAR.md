# The document grammar

The grammar of a Warrant's source files in the **SAS 1.1.0 atom format**:
the manifest, the atom header, structured atoms, the Markdown body's
headings, the obligation block, the gate citation, and roles. For each
construct it says what the SAS requires of any tool, what `war` does, and
where the two differ. OW-WAR-0122.

This is not the RC.3 human-first document format (footer TOML, units). That
has its own draft contract, `docs/sas/drafts/1.0.0-rc.3/format-contract.md`,
and its own Warrant, OW-WAR-0075. Nothing here decides which format is the
standard.

## How to read it

Two kinds of conformance, kept apart in every row:

- **Standard conformance** is what the SAS text requires of *any* tool that
  reads these files. It is derived only from the SAS. Each standard cell
  quotes the clause it rests on, or says **unspecified** where the SAS says
  nothing. "Accept" and "refuse" in a standard cell are what the quoted text
  demands, no more.
- **Tool conformance** is what `war` accepts and refuses, and the diagnostic
  rule it reports. A tool may be stricter than the standard where the
  standard is unspecified; where it contradicts the standard, the row names a
  divergence (`DV-n`, below).

Each table has five columns: the construct, the SAS clause, the standard
cell, the `war` cell and the plant. A rule is cited as the word *rule*
and its id in backticks. A plant is written `G-` and a letter and number,
and lives in `conformance/plants.d/57-grammar.sh`, which runs
it on a scratch program and checks which rule fired and for what. Where no
plant exists the row cites the source line instead. The plant file ends with
a drift control, `G-D1`: it fails if this document cites a rule id or a
plant id that no plant produced. `G-D2` shows that control failing on a
planted citation.

## 1. The manifest

`manifest.toml`, TOML, schema `oh.war/manifest/v1` (§61).

| construct | SAS | standard | `war` | plant |
|---|---|---|---|---|
| unique ordinals | §61.2, §91.2 test 8 | refuse: "Ordinals SHALL be unique within one composition" | refuse, rule `manifest.invalid` ("duplicate atom ordinal") | `G-M1` |
| a declared `path` atom that is not there | §91.2 test 7 | refuse when required: "Missing required atom fails." Unspecified when optional | refuse whether required or not, rule `atom.missing` | `G-M2`; optional case: source `crates/openwarrant-cli/src/repo.rs:395-404` (no `required` test) |
| an atom declared by `ref =` | §61.3 | refuse at authorization: "An authorized compilation SHALL resolve every atom and bound reference to an exact revision and digest." Unspecified offline | UNKNOWN, rule `atom.bound-unresolvable`: federation is not implemented, so neither pass nor error | `G-M9` |
| an unknown top-level key | §61, §87.2 | unspecified. §87.2 asks parsers to "preserve unknown extensions"; §61 names no extension point | accepted and ignored: `Manifest` has no `deny_unknown_fields`. The bytes survive only under `manifest_digest` (DV-4) | `G-M3` |

## 2. The atom header

The YAML frontmatter of a Markdown atom (§62). Example from §62, which `war`
refuses (DV-1):

```markdown
---
schema: oh.war/atom/v1
warrant_uuid: 019c8f2d-7b4d-7c41-9cb7-2636e5f582ea
atom_uuid: 019c8f34-f984-7208-89cb-e31620ad8804
role: intent
jurisdiction: authored
holder:
  kind: git
order: 10
classification: internal
---
```

### 2.1 The header against the manifest

Checked for every `.md` atom whose manifest role is not `adr`. ADR atoms
carry `adr_uuid` and `governs` instead of `warrant_uuid` and keep their own
check under the ADR loader (source `crates/openwarrant-cli/src/repo.rs:413`).
One diagnostic per key. A missing key names the key; a different value names
both values. `order` compares as a number, so `order: 05` restates ordinal 5.

| construct | SAS | standard | `war` | plant |
|---|---|---|---|---|
| the scaffold's header, restating its entry | §62 | unspecified: the example shows the keys, and no clause says what they must equal | accept | `G-H0` |
| `role` differs from the manifest entry's role | §62, §61.1 | unspecified. §61.1: "The manifest defines composition" | refuse, rule `atom.header`, naming both roles (DV-7) | `G-H1` |
| `warrant_uuid` names another Warrant | §62 | unspecified | refuse, rule `atom.header`, naming both uuids | `G-H2` |
| `order` differs from the manifest ordinal | §62, §61.2 | unspecified | refuse, rule `atom.header`, naming both | `G-H3` |
| `schema` missing (or not `oh.war/atom/v1`) | §62.3 | unspecified which fields are required: "Unknown required fields fail." | refuse, rule `atom.header`, naming the key | `G-H4` |

### 2.2 The other header keys

| construct | SAS | standard | `war` | plant |
|---|---|---|---|---|
| `jurisdiction` not one of three | §13.1–§13.3 | unspecified as a header value. §13 defines authored, bound and generated atoms | refuse, rule `atom.unknown-jurisdiction` | `G-J1` |
| `jurisdiction` other than §16.1 gives the role | §16.1 table, "Typical authority" | unspecified: the column says "typical" | refuse, rule `atom.jurisdiction-mismatch` | `G-J2` |
| `jurisdiction` absent | §13 | unspecified | accept, read as `authored` (source `crates/openwarrant-cli/src/repo.rs:422`) | `G-J3` |
| `holder`, as §62 writes it (nested) | §13, §62 | accept: §13 "Every atom or bound record SHALL declare its Source Holder", and §62's example is the only form given | refuse, rule `atom.frontmatter` ("an indented (nested) mapping") (DV-1) | `G-R10` |
| `atom_uuid` | §62 example | unspecified: in the example, in no clause | accepted and kept, never read. No atom in this corpus carries it (DV-6) | `G-P2` |
| `classification` | §62 example | unspecified | accepted and kept, never read (source `crates/openwarrant-cli/src/repo.rs:422`: only `jurisdiction` is read) | `G-P2` |
| an unknown key that is not namespaced | §62.3 | unspecified: "Unknown required fields fail." does not say whether an unknown key is required (U-002) | accepted and kept (DV-3) | `G-P2` (`atom_uuid`) |
| a namespaced key (`x.note: kept`) | §62.3, §91.1 test 5 | accept and preserve: "Namespaced optional fields are preserved." | accepted, kept in the source bytes the IR's `atom_source_digest` binds (§62.2); not an IR field, and stripped from `WAR.md` (DV-12) | `G-P3` |

### 2.3 The reader (OW-ADR-0002)

§62 says "YAML frontmatter" and names no subset, so standard conformance
accepts what YAML accepts. `war` reads a restricted subset:
`key: scalar` (plain, `"double"` or `'single'` quoted) and `key:` followed
by `- item` lines, top level only, `#` comments and blank lines allowed, a
UTF-8 BOM tolerated (source `crates/openwarrant-core/src/frontmatter.rs:188`).
Everything else is refused by name (DV-2).

| construct | SAS | standard | `war` | plant |
|---|---|---|---|---|
| no opening `---` fence | §62 | unspecified: "Markdown with YAML frontmatter" | refuse, rule `atom.frontmatter` | `G-R9` |
| an anchor, `&a` | §62 | accept: "YAML frontmatter" | refuse, rule `atom.frontmatter` ("a YAML anchor") | `G-R1` |
| an alias, `*a` | §62 | accept: "YAML frontmatter" | refuse, rule `atom.frontmatter` ("a YAML alias") | `G-R2` |
| a tag, `!!str` | §62 | accept: "YAML frontmatter" | refuse, rule `atom.frontmatter` ("a YAML tag") | `G-R3` |
| a flow sequence, `[a]` | §62 | accept: "YAML frontmatter" | refuse, rule `atom.frontmatter` ("a flow sequence") | `G-R4` |
| a flow mapping, `{k: v}` | §62 | accept: "YAML frontmatter" | refuse, rule `atom.frontmatter` ("a flow mapping") | `G-R5` |
| a block scalar, `\|` | §62 | accept: "YAML frontmatter" | refuse, rule `atom.frontmatter` ("a block scalar") | `G-R6` |
| a folded scalar, `>` | §62 | accept: "YAML frontmatter" | refuse, rule `atom.frontmatter` ("a folded block scalar") | `G-R7` |
| a duplicate key | §62 | refuse: "YAML frontmatter", and YAML requires a mapping's keys to be unique | refuse, rule `atom.frontmatter` ("duplicate key") | `G-R8` |
| a nested mapping | §62 example | accept: the example nests `holder` | refuse, rule `atom.frontmatter` ("an indented (nested) mapping") (DV-1) | `G-R10` |
| §62's example with `holder` removed | §62 | accept | the reader accepts it. Its `warrant_uuid` names the example's Warrant, so rule `atom.header` refuses it and nothing else does | `G-P1` |

## 3. Structured atoms

`.yaml` atoms, today only the milestones atom (§62.1), read by the second
restricted reader (OW-ADR-0003): top-level `key: scalar`, or `key:` over a
block sequence of flat mappings whose values are scalars or flow sequences
of scalars. Two levels, no more. The milestones atom is found by role, not
extension, and always goes to that reader (source
`crates/openwarrant-cli/src/check.rs:1604-1605`).

| construct | SAS | standard | `war` | plant |
|---|---|---|---|---|
| an anchor | §62.1 | accept: "MAY use YAML or canonical JSON source" | refuse, rule `milestones.invalid` ("a YAML anchor") | `G-S1` |
| a mapping below a sequence item | §62.1 | accept: YAML | refuse, rule `milestones.invalid` ("a mapping nested below a sequence item") | `G-S2` |
| a duplicate key | §62.1 | refuse: YAML requires unique keys | refuse, rule `milestones.invalid` ("duplicate key") | `G-S3` |
| canonical JSON source (`.json`) | §62.1 | accept: "MAY use YAML or canonical JSON source" | refuse, rule `milestones.invalid` (DV-8) | `G-S4` |

## 4. Markdown body headings

The SAS gives no heading grammar. `war` uses one for a stage's
`context_sections` (`<atom>#<heading>`), when it compiles a dispatch: an ATX
heading is 1–6 `#` then a space, matched by its trimmed text,
case-sensitively. Lines inside a ```` ``` ```` fence are not headings
(source `crates/openwarrant-core/src/sections.rs:8-28`). `~~~` fences and
setext headings are not recognised (same lines). `war check` does not
resolve `context_sections`; a missing section is found at `war dispatch`.

| construct | SAS | standard | `war` | plant |
|---|---|---|---|---|
| a section selected by its exact heading | none | unspecified | accept | `G-B1` |
| a `## heading` inside a fence | none | unspecified | not a heading: rule `dispatch.section-missing` | `G-B2` |
| the heading in another case | none | unspecified | no match: rule `dispatch.section-missing` | `G-B3` |

## 5. The obligation block

§38.1 requires bounded obligations, and §38.2 gives their schema as YAML.
The SAS gives no Markdown form. `war` reads one, fitted to the corpus
(`crates/openwarrant-core/src/obligation.rs`) and placed under tool
conformance (U-003, DV-5):

```markdown
### OBL-001 — the statement
- **scope:** what the claim covers, and what it does not.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** what would settle it.
```

- An `### ` heading whose first token starts `OBL-` opens an obligation. The
  statement follows an em dash, or ` - `. Any other `### ` heading, or any
  `## ` heading, closes it (source `obligation.rs:407-439`).
- Bullets are `- **key:** value`, the key case-insensitive. `scope` and
  `evidence` are required. `note`, `disposition`, `origin`, `admissibility`
  and `scope kind` are read. Any other bullet is prose (source
  `obligation.rs:443-496`). A line that is not a bullet continues a
  `scope` or `evidence` value and is dropped after any other bullet
  (`obligation.rs:499-511`).
- Lines are read trimmed, so an **indented** `- **word:**` bullet inside an
  evidence list is a new key, not part of the evidence: an unknown word
  ends the `evidence` value there, silently, and every line after it is
  dropped (DV-14).

| construct | SAS | standard | `war` | plant |
|---|---|---|---|---|
| no `scope` bullet | §38.4 | unspecified as Markdown. §38.4 bounds claims; §38.2's schema carries `claim_scope` | refuse, rule `obligations.invalid` ("declares no scope") | `G-O1` |
| no `evidence` bullet | §38.2 | unspecified | refuse, rule `obligations.invalid` ("declares no evidence") | `G-O2` |
| a duplicate obligation id | §38.2 | unspecified as Markdown | refuse, rule `obligations.invalid` ("duplicate obligation id") | `G-O3` |
| ` - ` in place of the em dash | none | unspecified | accept; the obligation is parsed (the milestone's `obligation_refs` still resolve) | `G-O4` |

## 6. The gate citation

An obligation cites its gate in a `- **gate:**` (or `- **gates:**`) bullet.
Tokens split on commas and whitespace, backticks and a trailing full stop
stripped (source `crates/openwarrant-core/src/gate.rs:737-760`). A `gate://`
URI in prose is prose, not a citation.

| construct | SAS | standard | `war` | plant |
|---|---|---|---|---|
| a gate not in the registry | §43.5 | unspecified as Markdown. §43.5 makes a binding an object | refuse, rule `gate.unresolved` ("not in the registry") | `G-G1` |
| §105's URI form, `gate://<id>/<version>` | §105 | accept: "Recommended logical forms: … `gate://<gate-id>/<version>`" | refuse, rule `gate.unresolved` ("malformed gate URI"); `war` reads `gate://<id>@<version>` (DV-11) | `G-G2` |

## 7. Roles and extension roles

The core roles are `control`, `intent`, `basis`, `adr`, `work_order`,
`milestones`, `execution`, `assurance`, `resolution`, `validation` and
`relations_and_integrity` (source `crates/openwarrant-core/src/role.rs:110-122`).
An extension role is namespaced: it contains a `.` with text on both sides.

| construct | SAS | standard | `war` | plant |
|---|---|---|---|---|
| an unknown role, required | §16.4, §91.2 test 9 | refuse: "Unknown required roles SHALL fail closed." | refuse, rule `manifest.invalid` | `G-M4` |
| an unknown role, optional, not namespaced | §16.4 | unspecified | refuse, rule `manifest.invalid` (DV-10) | `G-M5` |
| a namespaced role, required | §16.4 | refuse: an unknown required role | refuse, rule `manifest.invalid` | `G-M6` |
| a namespaced role, optional | §16.4 | accept and preserve: "Unknown optional namespaced roles SHALL be preserved in the canonical export" | accept; the role is in the compiled IR | `G-M8` |
| `decisions`, §16.1's name for ordinal 30 | §16.1, §16.2, §61 | unspecified: §16.1's table names the role `decisions`, while §16.2 and §61's example write `role = "adr"` | refuse, rule `manifest.invalid`; `adr` is the role, `decisions` its section (DV-9) | `G-M7` |

## Divergences

Each is a place where `war` and the SAS text disagree, or where `war` is
stricter than a text that says nothing. The **proposed SAS text** under each
is a proposal only. Adopting it is a separate SAS revision, the owner's act;
nothing here amends `docs/sas/`. Where the tool would have to change to
match, a later Warrant does that. OW-ADR-0002 and OW-ADR-0003 stand.

**DV-1. §62's example is refused.** The example nests `holder:` over
`kind: git`, and the frontmatter subset has no nested mappings (`G-R10`).
§13 makes the holder a SHALL, so the SAS's only form for a required field is
one `war` cannot read. No atom in this corpus declares a holder (OW-ADR-0008
narrowed §91.2 test 14 for this reason).

> *Proposed SAS text, §62:* replace the example's `holder:` / `kind: git`
> lines with `holder: git`, and add: "Frontmatter values are scalars or
> flat lists of scalars; a structured value is written as a scalar or a
> namespaced key (`holder.kind: git`), never as a nested mapping."

**DV-2. Frontmatter is a subset of YAML, not YAML.** Anchors, aliases,
tags, flow collections and block scalars are refused (`G-R1`–`G-R7`), on
purpose (OW-ADR-0002: implicit typing and aliasing would reach the
validator). §62 says only "YAML frontmatter".

> *Proposed SAS text, §62:* "Frontmatter is the restricted subset defined in
> the grammar appendix: top-level `key: scalar` pairs and `key:` over a
> block list of scalars, with plain or quoted scalars. A conforming tool
> SHALL refuse anchors, aliases, tags, flow collections, block scalars,
> nested mappings and duplicate keys, naming the construct."

**DV-3. Unknown keys that are not namespaced are kept.** `war` keeps every
unknown key (`G-P2`). §62.3's "Unknown required fields fail" never says
which fields are required, so it cannot say whether an unknown key is one
(U-002). `war` adds no rule for it here.

> *Proposed SAS text, §62.3:* "For `oh.war/atom/v1` the required fields are
> `schema`, `warrant_uuid`, `role` and `order`; the known optional fields
> are `jurisdiction`, `classification`, `atom_uuid` and `holder`. A key that
> is neither known nor namespaced SHALL fail. Namespaced optional fields are
> preserved."

**DV-4. Unknown manifest keys are ignored.** A manifest with an extra key
compiles (`G-M3`). §61 names no extension point, and §87.2 asks parsers to
"preserve unknown extensions". The bytes are kept under `manifest_digest`;
no IR field carries the key.

> *Proposed SAS text, §61:* "A manifest key the schema does not define
> SHALL fail unless namespaced. A namespaced key is preserved in the
> manifest's source bytes and its digest."

**DV-5. The SAS does not specify the obligation's Markdown form.** §38.2
gives a YAML schema; every Warrant in this corpus writes the `### OBL-NNN`
form of section 5, and `war` reads only that. It is tool conformance, not
standard (U-003).

> *Proposed SAS text, §38.2:* "An assurance atom MAY carry obligations in
> the Markdown form: a level-3 heading `OBL-NNN — statement`, followed by
> `- **scope:**`, `- **gate:**` and `- **evidence:**` bullets. `scope` and
> `evidence` are required. It lowers to the schema above."

**DV-6. `atom_uuid` is in §62's example and in no atom.** No atom in this
corpus carries it, and `war` neither requires nor reads it (`G-P2`). An
atom's identity today is its Warrant, its role and its ordinal.

> *Proposed SAS text, §62:* either remove `atom_uuid` from the example, or
> add: "`atom_uuid` is optional; when present it identifies the atom across
> revisions and SHALL be unique within the Warrant."

**DV-7. The header must restate the manifest; the SAS does not say so.**
`war` refuses a header whose `schema`, `warrant_uuid`, `role` or `order`
disagrees with the manifest entry (rule `atom.header`, `G-H1`–`G-H4`).
Without it, an atom copied from another Warrant compiles as whatever the
manifest says it is. The SAS is silent, so this is stricter, not contrary.

> *Proposed SAS text, §62.3:* "A header's `warrant_uuid`, `role` and `order`
> SHALL equal the manifest's `uuid` and the declaring entry's role and
> ordinal. A difference fails, naming the key and both values."

**DV-8. Canonical JSON structured atoms are refused.** §62.1 permits them;
`war` reads every milestones atom with the YAML-subset reader
(`G-S4`).

> *Proposed SAS text, §62.1:* "Machine-dense atoms such as milestone graphs
> use the structured subset defined in the grammar appendix. Canonical JSON
> source is reserved for a later schema version."

**DV-9. §16.1 and §61 name the ADR role differently.** §16.1's table names
the role at ordinal 30 `decisions`; §16.2 and §61's example write
`role = "adr"`. `war` accepts `adr` and composes it into a `decisions`
section; `decisions` as a manifest role is refused (`G-M7`).

> *Proposed SAS text, §16.1:* the row for ordinal 30 reads role `adr`, with
> the note "composed into the `decisions` section".

**DV-10. An unknown optional role must be namespaced.** §16.4 fails an
unknown required role and preserves an unknown optional namespaced one. It
says nothing of an unknown optional role that is not namespaced; `war`
refuses it (`G-M5`).

> *Proposed SAS text, §16.4:* "An unknown role that is not namespaced SHALL
> fail closed, whether or not it is required."

**DV-11. The gate URI uses `@`, not `/`.** §105 recommends
`gate://<gate-id>/<version>`; `war` reads `gate://<id>@<version>` and
refuses the slash form (`G-G2`). Every gate citation in this corpus uses
`@`, as do §105's `atom://` and `kf://` forms.

> *Proposed SAS text, §105:* `gate://<gate-id>@<version>`.

**DV-12. A namespaced header key survives as bytes, not as a field.**
§91.1 test 5: "Optional namespaced extensions survive round trip." `war`
keeps the key in the atom's exact bytes, which the IR binds by
`atom_source_digest` (§62.2), and `WAR.md` renders the atom without its
header (`G-P3`). No IR field carries the key's value.

> *Proposed SAS text, §91.1 test 5:* "Optional namespaced extensions
> survive round trip: in the preserved source bytes (§62.2) at least, and
> in the canonical IR where the schema defines an extension field."

**DV-13. An optional atom that is missing is refused.** §91.2 test 7 fails
a missing *required* atom. `war` refuses any declared `path` it cannot
read (rule `atom.missing`, `G-M2`); the `required` flag is not consulted
(source `crates/openwarrant-cli/src/repo.rs:395-404`). Stricter, not contrary.

> *Proposed SAS text, §61:* "A declared `path` SHALL exist. `required =
> false` means a profile does not demand the role, not that the file may be
> absent."

**DV-14. An unknown bold key inside evidence ends the evidence silently.**
A nested bullet such as `  - **refusal:** …` under `- **evidence:**` is read
as a new key (lines are trimmed), and an unknown key ends the value: the
refusal and everything after it vanish from the compiled contract with no
finding. Found 2026-09-24 in the OW-WAR-0141 draft, whose seven paired
refusals would all have been dropped. Write nested evidence bullets without
bold keys (`  - refusal: …`). `war` should refuse or warn on an unknown key
inside an obligation rather than drop text; that is a tool change for a
later Warrant, not this document's.

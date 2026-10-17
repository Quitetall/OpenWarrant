---
schema: oh.war/atom/v1
adr_uuid: 01a12a33-8ae9-7094-a5c6-76e700d28941
local_alias: OW-ADR-0032
role: adr
jurisdiction: bound
order: 30
classification: internal
status: accepted
---

# ADR OW-0032: OpenWarrant Format 1.0 is the standard; the SAS becomes rationale

## Status

Accepted by the owner on 2026-10-16. Directed by the owner on 2026-10-11:

> "I want to make a durable document standard, not really the software
> around it. The openwarrant style documents are what are read efficiently
> by Agents and engineered well so context is high quality."

The same day the owner directed: settle the standard's identity, write the
format 1.0 document, write JSON schemas, put the content in the canonical
JSON, add extension rules, allow a single-file Warrant with an optional
manifest, make the format cheap for agents and fit a deterministic context
compiler, build a conformance corpus, and license the spec text.

So the decisions below are the owner's direction, not open options. The
performer drafted this record and does not accept it: an accepted ADR here
records the owner's own act (OW-ADR-0017), so the status stays `proposed`
until the owner changes it. It governs no Warrant yet. The text it decides
is `docs/format/FORMAT-1.0.md` (Draft).

## Context

A gap analysis of `main` on 2026-10-11 found:

- **No single canonical standard.**
  - The accepted SAS revision is 1.3.0, but the SAS file's header says
    "Version 1.0.0, Proposed for 1.0". That header is inside the accepted
    bytes.
  - `revisions/1.1.0.toml` is accepted and signed, while
    `docs/sas/drafts/README.md` called 1.1.0 withdrawn.
  - RC.3 (`oh.war/document/1.0.0-rc.3`) is called the product-direction
    candidate.
- **Three live authored grammars.**
  - The SAS atom format: manifest, YAML-subset frontmatter, and
    `### OBL-NNN —` bullets. Tickets are a variant of it.
  - `oh.war/records/v1`. Its grammar exists only in `docs/TYPES.md` and the
    Rust code.
  - RC.3's TOML footer.
  - The SAS never mentions records or tickets.
- **Fourteen tool/text divergences**, DV-1 to DV-14 in `docs/GRAMMAR.md`.
- **The canonical JSON carries identity, not content.**
  - `WAR.json` holds digests; the obligation text appears in it 0 times.
  - The 26 schemas describe Rust structs, not authored files.
  - `COMPATIBILITY.md` promises schemas that do not exist.
- **Federation sits in the normative core.** Knowledge Fabric, Katana,
  Liminal and BLUT are all there, and every manifest carries
  `enterprise_id = ""`.
- **Cost to agents.**
  - One unit of work spans 25–42 files.
  - Atoms carry one `§` reference per ~680 bytes.
  - There is no summary tier.
  - Journal payloads are JSON strings inside JSON.

Strengths to keep:

- record ids with `path:line · revision` provenance;
- declared, budgeted selection that refuses rather than truncates;
- bounded obligations paired with refusals;
- UNKNOWN as a third value;
- recorded verifier independence;
- the frozen/additive/v2 compatibility policy;
- RFC 8785 digests;
- interop with Beads, OpenSpec and Spec Kit.

## Decision

### D1. Identity and versioning

- `docs/format/FORMAT-1.0.md` is the one normative document. It is
  self-contained, written in RFC 2119 keywords, and depends on no `§`.
- Every document declares its version in its own frontmatter:
  `format: openwarrant/1.0`.
  - A patch changes wording only.
  - A minor only adds.
  - A major is a new `format` value, read forever.
  - A reader meeting a newer major reports UNKNOWN, never PASS.
- SAS 1.0.0–1.3.0 and RC.1–RC.3 are relabelled **rationale and history**:
  - Their files and revision records are not edited.
  - No signed digest moves.
  - The relabelling is recorded in `docs/format/README.md` and here.
  - `docs/sas/drafts/README.md` now states the 1.1.0 facts accurately: it
    was accepted and signed, its release plan was later set aside, and no
    withdrawal act was recorded.
- RC.3's footer format is not adopted.

### D2. One authored grammar

- `oh.war/records/v1` is extended into the Format 1.0 grammar:
  - record headings carry a required title;
  - records gain an attribute block of fields, relations and verbatim
    continuations;
  - a plain `## Name` opens a section;
  - obligations, milestones, stages and items are records.
- A Warrant is **one file** with frontmatter, sections and records.
  - Its manifest is optional, never lists atoms, and never restates a
    frontmatter key.
  - Its evidence (journal, gate runs, bundles, signatures) lives beside the
    file and is never needed to read its meaning.
- A ticket is a work document with the `ticket` profile.
- Owner decision 14 stands: directory Warrants, ticket stores, ADR atoms,
  the roadmap and `records/v1` atoms are **legacy encodings**.
  - They are read forever and lowered to the same model.
  - Nothing is migrated, and no signed digest moves.

### D3. Core and extensions

The core is small: documents, records, sections, relations, obligations,
items, computed states, the model, digests and the agent-context rules.
Authority (authorization, verification, resolution, signatures) is an
optional profile.

Unknown names:

- **Dotted names** (`x.` private, a reverse domain, or a registered
  prefix) are must-ignore. They are carried in the model under `ext`.
- **Undotted names** the format and the profile do not define are refused,
  unless the document declares a newer minor. This catches typos.
- **Monotonic rule:** ignoring a name never turns a refusal into an allow.
  An extension whose absence would loosen a check is critical. It is
  listed in `requires`, and a reader that does not know it reports UNKNOWN.

Federation (Knowledge Fabric, Katana, Liminal, BLUT) moves to an optional
profile under the prefixes `kf`, `katana`, `liminal` and `blut`.
`enterprise_id` leaves the core and becomes `kf.enterprise_id`.

### D4. Canonical JSON carries content; digests cover meaning

- `oh.war/model/v1` grows additively by `format`, `title`, `document`,
  `section`, `line`, `span`, `content_digest`, `fields`, `body` and `ext`.
  Consumers must ignore members they do not know.
- Each record keeps its byte `revision`, which existing signatures bind.
- Each record gains `content_digest`: RFC 8785 over a domain-separated
  payload of its meaning. A record therefore has the same digest in every
  encoding.
- `war context` emits `oh.war/context/v1`. It is a deterministic function
  of the documents:
  - declared walk;
  - fixed order;
  - summary-first tiers;
  - a byte budget that refuses or names what did not fit, and never cuts a
    record.
- A generated `llms.txt` index gives one line per record.

### D5. Licence and governance

- The spec text is licensed CC-BY-4.0 (`docs/format/LICENSE`, and a
  paragraph in `NOTICE`).
- The grammar, schemas, conformance corpus and code stay Apache-2.0
  (OW-ADR-0017).
- `docs/format/GOVERNANCE.md` sets the process:
  - the owner is the editor;
  - a change is an issue, then a PR with a proposed ADR and conformance
    cases, then at least 7 days for comment, then the editor's decision;
  - versions follow D1.

## Options considered

- **Make the SAS the standard and fix its divergences.** Rejected. It is
  ~37k tokens of architecture, federation and CLI, and its accepted bytes
  cannot be edited.
- **Adopt RC.3's footer format.** Rejected. It would add a third grammar
  where one is wanted, and no corpus document uses it.
- **Keep the directory-atom format normative.** Rejected. It spreads one
  unit across 25–42 files and restates the manifest in every header
  (DV-7). It survives as a legacy encoding.
- **Must-understand for every unknown key.** Rejected. Every minor version
  would break every older reader.
- **Must-ignore for every unknown key.** Rejected. Typos would pass
  silently (DV-3, DV-14). The undotted/dotted split gives both.
- **A new `oh.war/model/v2`.** Rejected for now. The model is generated,
  never signed, and the additions are additive.
- **Apache-2.0 for the text too.** Rejected. A licence written for code
  fits prose and redistribution of a standard poorly. CC-BY-4.0 is the
  usual choice for open specifications.

## Consequences

`war` owes the following, each its own work:

- the single-file reader;
- the attribute block and titled headings;
- the obligation record;
- refusal of unknown undotted keys;
- model content and `content_digest`;
- `war context` and the `llms.txt` index;
- profile declarations for frontmatter keys and sections;
- the legacy reader's warning on an unknown bold key (DV-14).

Each divergence DV-1 to DV-14 now has one resolution (FORMAT-1.0 Appendix
B).

New artifacts are needed: JSON schemas generated for the authored form,
not only for structs, and `conformance/format/` cases derived from the
text.

Existing Warrants, tickets, ADRs and signatures are untouched and read
as before.

The SAS stays accepted at 1.3.0 as rationale. New Warrants cite record ids
and Format sections, not SAS `§` numbers.

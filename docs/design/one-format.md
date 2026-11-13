# One format, one model, masters and views

A Format 1.0 document is the unit. Every act reads it through one loader,
`crate::subject`, which is also the only place the layout is decided: the roots,
which file in a directory is the document, where state goes, and where new work is
written.

## Layouts

| layout | the document | its state (§4.3) |
|---|---|---|
| master | `docs/work/<id>/<id>-<slug>.md` | `docs/work/<id>/` |
| file (older) | `docs/warrants/<alias>.md` | `docs/warrants/<alias>/` |
| directory (older) | `docs/warrants/<alias>/warrant.md` or `<alias>-<slug>.md` | the same directory |
| legacy | `docs/warrants/<alias>/manifest.toml` and `atoms/` | the same directory |

Masters are what people and agents edit, and what the context compiler reads.
The state directory holds what only a directory can: `deliverables.toml`,
evidence and receipts, `authorization.toml`, verdicts, `resolution.toml`,
amendments, `generated/`, the journal. Nothing signed moves: `docs/warrants/`,
`docs/tickets/`, `docs/sas/` and `docs/roadmap/` stay readable where they are.

## Identity

- The id is identity. New work orders get `w-<hex>` and new documents `d-<hex>`,
  the head of the SHA-256 of a fresh UUIDv7, so two branches never take the same
  one. The prefix shows the kind, never the tier, so a promote changes no id.
  `t-` ids and `OW-WAR-NNNN` aliases stay valid and are never renamed. `spec` and
  `roadmap` are the only fixed ids.
- The slug is cosmetic. It is in the file name only. The directory is named by
  the id, so renaming the slug moves no state and no digest. The loader finds an
  item by id whatever its slug, and confirms it by the frontmatter `alias`. Two
  documents claiming one id is refused (`format.document-duplicate`).
- A human number is a display ordinal. Views read it from an append-only index
  that the main-only views job writes, and fall back to the id. It is never
  computed from dates and never stored in a file name.

## Sections and atoms

`subject::load` lowers a document, in memory, to the Compilation Basis the acts
already use. Nothing is written.

| document | basis |
|---|---|
| frontmatter `alias`, `uuid`, `profile`, `assurance_level`, `# ` title | the manifest fields |
| the document's exact bytes | `manifest_bytes`, so `manifest_digest` covers every byte |
| `## Intent`, `## Basis`, `## Work order` (`## Decision` for `decision`) | atoms holding the section's exact span |
| `## Assurance` and its obligation records | atom `assurance`, with the records restated as `### OBL-NNN — title` blocks with `- **field:**` bullets |
| milestone records | the milestones atom; each obligation `part_of` one is in its `obligation_refs` |
| stage records | not lowered (`subject.stages-unlowered`, UNKNOWN) |

Sources are ids, not paths (`w-3c16`, `w-3c16#intent`), so the contract digest is
the same in every layout and under any slug.

## What a signature covers

An act's response echoes the contract digest, the same as for a legacy Warrant.
For a document, that digest covers its exact bytes, its identity and the lowered
atoms. Any edit after authorization moves it: the authorization stops matching,
`war resolve` refuses it, and a new authorization is needed. Moving the bytes, by
expanding or renaming the slug, changes nothing.

Progress never rewrites a document once an act is recorded (`authorization.toml`
or `resolution.toml` in its state). `ticket::rewrite`, the store's one document
writer, refuses such a write (`subject.signed-bytes`). Ticks, notes and check
results belong in `<id>/` state, and the checklist people see is a projection of
it. No command writes to a non-ticket document today, so this guard has nothing
to refuse yet. Plain Tickets are unchanged.

## Which acts need a directory

None of authorize, evidence record, verify, resolve, compile, check, status or
pins. Each writes into the state directory. Features that only the legacy
encoding can express stay legacy: namespaced extension roles, atoms bound to a
file outside the Warrant, dispatchable stages, `--parent` revisions, presets'
atom headings, and `plan --apply`'s atom operations. Splitting a document into
legacy atoms re-encodes it, and C.3 says that lapses its acts, so it is not built.

## Expand

`war plan expand <id>` moves `docs/warrants/<id>.md` to
`docs/warrants/<id>/<id>-<slug>.md`, beside its state. It keeps the same id and
the same bytes, so the contract digest is unchanged and every signature stands.
It refuses a document already in a directory. Moving older work into `docs/work/`
is the migration tool's job, not this one.

## Where new work goes

`war plan new <title>` writes a master by default. `--format 1.0` writes
`docs/warrants/<alias>.md`, and `--format legacy`, `--preset`, `--parent` and the
old spelling `war new` (which keeps its bytes) write the legacy directory.
`war plan --apply` and `war init --program` still write legacy directories,
because their inputs are atom operations and atom templates.

## Normative text

FORMAT-1.0 revision 4 (OW-ADR-0035, proposed until the editor accepts it) writes
what this design needed: §5's `w-`, `d-` and fixed ids, slugs and display
ordinals; §4.1 and §4.3's masters, state directories and views; C.3's contract
digest beside the subject digest; §4.1's split masters; and §4.6's ladder. A
split master is defined there and not built: no tool writes or joins parts.

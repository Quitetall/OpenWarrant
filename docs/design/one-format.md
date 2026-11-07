# One format, two layouts

A work document is the unit (FORMAT-1.0 §4.1). It has two layouts, and every act
reads both through one loader, `crate::subject`.

| layout | the document | its state (§4.3) |
|---|---|---|
| file | `<root>/<id>.md` | `<root>/<id>/` |
| directory | `<root>/<id>/warrant.md` | the same directory |

The state directory holds what only a directory can: `deliverables.toml`,
evidence and receipts, `authorization.toml`, `resolution.toml`, verifications,
amendments, `generated/`, runtime receipts, the journal. It exists beside a file
document as soon as an act writes state, so no act needs the document moved.
A legacy Warrant (`manifest.toml` and `atoms/`) is a third encoding, read in place
forever and never rewritten.

## Sections and atoms

`subject::lower` turns a document into the Compilation Basis every act already
consumes, in memory. Nothing is written.

| document | basis |
|---|---|
| frontmatter `alias`, `uuid`, `profile`, `assurance_level`, `# ` title | the manifest fields |
| the document's exact bytes | `manifest_bytes` (so `manifest_digest` is over every byte) |
| `## Intent`, `## Basis`, `## Work order` | atoms `intent`, `basis`, `work_order`: the section's exact span |
| `## Assurance` and its obligation records | atom `assurance`, the records restated as `### OBL-NNN — title` with `- **field:**` bullets |
| any other section | not an atom; still inside `manifest_bytes` |

Sources are ids, not paths (`OW-WAR-0007`, `OW-WAR-0007#S-intent`), so the basis,
and with it the contract digest, is the same in both layouts.

## What a signature covers

An act's response echoes the contract digest, as for a legacy Warrant. For a
document that digest is over its exact bytes, its identity and the lowered atoms.
Any edit after authorization therefore moves the digest: the authorization stops
matching the contract, `war resolve` refuses it, and `war check` reports it as it
does for a legacy Warrant. Amending means a new authorization over the new bytes.
Moving the bytes between layouts moves nothing, because no path is in the digest.

Progress never rewrites a signed document. Once an act is recorded for a subject
(`authorization.toml` or `resolution.toml` in its state directory), a tick, note
or check result is refused as a document write (`subject.signed-bytes`). It
belongs in `<id>/` state, and the checklist shown to people is a projection of
that state. Plain Tickets with no act behave as before. The other choice, a
digest over a canonical form without tick markers, would need a new FORMAT rule.

## Which acts need a directory

None of authorize, evidence record, verify, resolve, compile, check or status
needs one: their state goes in the state directory. The layout question remains
for features whose input only the legacy encoding can express, which a document
does not have:

- a namespaced extension role or an atom `bound` to a file outside the Warrant
  (an ADR atom, a `ref =` atom);
- a structured `45-milestones.yaml` with dispatchable stages;
- `plan --apply` operations that revise an existing atom.

Each of these refuses a document and names `war plan expand <id>`.

## The expand step

`war plan expand <id>` moves `<root>/<id>.md` to `<root>/<id>/warrant.md`, byte for
byte, beside any state already there. Same id, same bytes, same contract digest, so
authorization, verdicts and resolution stand. It refuses when the directory already
holds a document or a manifest. Splitting the sections into legacy atom files is a
re-encoding: C.3 says the act lapses when a covered revision changes. So `--atoms`
is offered only while no act is recorded, and is not built in this slice.

## Where new work goes

`war plan new`, `war plan --apply` and `war init --program` write one file by
default; `--format legacy` (or a profile with extension roles) still writes the
legacy directory. `war create` is unchanged.

## A single root (not done here)

The loader takes a list of work roots, today `[paths] warrants` and `[tickets] dir`,
and finds an id in any of them; a level never chooses the root, so a level change
never moves a file. One configured root needs `[paths] work`, new work written
there, and the old roots kept in the list. Identity is the `uuid` (UUIDv7) and the
id; `OW-WAR-NNNN` is allocated by ordinal and is not merge-safe, so in a single root
the number should become a display ordinal over a hex id like a ticket's. One
`docs/generated/` instead of per-Warrant `generated/` needs `check --generated`
(check.rs, pinned) and the compiler changed.

What must stay where it is, because signed records or resolved pins name it:

- every legacy `docs/warrants/<alias>/`: `manifest_source` and atom sources are
  repository-relative paths inside its contract digest, so a move lapses its
  authorization and stales its resolution;
- `docs/authority/responses/*` and their `.sig` files;
- every `target_ref` in a resolved `deliverables.toml` (`war admin pins --resolved-only`);
- `docs/sas/` and its revisions; `docs/tickets/<id>/journal.jsonl` and sign-offs
  naming items.

## Normative text that waits for a signing sitting

- C.3 defines a Format 1.0 subject digest over `[id, revision, content_digest]`.
  Responses still carry the contract digest, which binds every byte and is
  stronger, but is not that value. Recording the C.3 digest beside it is a schema
  change (`oh.war/*-response/v1`).
- §4.1 says nothing of a document's sections split across files. Atoms for a
  Format 1.0 document need wording, or stay legacy-only.
- §4.3 should name the state directory as the place for every act's records.
- A single work root and display ordinals change §5 and §4.1's paths.

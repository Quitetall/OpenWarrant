# OpenWarrant Format

This directory holds the **OpenWarrant Format**, the document standard for
OpenWarrant documents. The `war` tool is one implementation of it.

| file | what it is |
|---|---|
| [FORMAT-1.0.md](FORMAT-1.0.md) | the standard: grammar, model, canonical JSON, digests, extensions, agent context, conformance |
| [GOVERNANCE.md](GOVERNANCE.md) | who edits it, how a change is proposed, how versions move |
| [LICENSE](LICENSE) | CC-BY-4.0, for the text of this directory |

## Status

**Draft, `1.0.0-draft.2`.** The owner set the direction on 2026-10-11, and
it is recorded as [OW-ADR-0032](../adr/atoms/OW-ADR-0032-format-1-0-is-the-standard.md).
That ADR is `proposed` until the owner accepts it; until then nothing here
is in force.

`war` implements this draft: it reads single-file and directory Warrants
and tickets, emits the content-carrying model and `war context`, and
writes `docs/generated/llms.txt`. The JSON schemas are in
[`schemas/format/1.0/`](../../schemas/format/), and the corpus in
[`conformance/format/`](../../conformance/format/) passes under `war` (in the
gate, plant 183) and under an independent Python reader,
`conformance/format/reader.py`.

Draft 2 settles what building those found unclear: item and record-document
records, frontmatter relations, containment outside `content_digest`, the
legacy lowering table, one finding per fault, an index that lists work
documents by their own record, and items not repeated in a context packet.

## What is normative, and what is history

Once OW-ADR-0032 is accepted:

| document | standing |
|---|---|
| `docs/format/FORMAT-1.0.md` | **normative** |
| `docs/sas/WAR_Software_Architecture_Specification.md`, revisions 1.0.0, 1.1.0, 1.1.1, 1.2.0, 1.3.0 | rationale and history. Each stays accepted at its recorded digest, and nothing is edited |
| `docs/sas/drafts/1.0.0-rc.1` to `rc.3` | history. RC.3's footer format is not adopted |
| `docs/GRAMMAR.md`, `docs/TYPES.md`, `docs/TICKETS.md` | what `war` does today. Where they differ from the standard, the standard governs and the tool owes the change (FORMAT-1.0 Appendix B) |

The SAS files are not changed to say this. Their bytes are bound by
accepted digests, so this table is where the relabelling is recorded. The
SAS file's own header still says "Version 1.0.0" and "Proposed for 1.0".
That header is part of the accepted 1.3.0 bytes, and it stays.

## How to comment

- For a question or a defect, open a GitHub issue titled `format: …` and
  name the section (`FORMAT-1.0 §3.5`).
- For a change, open a pull request that edits `FORMAT-1.0.md` and adds a
  proposed ADR atom under `docs/adr/atoms/`. See [GOVERNANCE.md](GOVERNANCE.md).
- Before merge, a normative change adds or changes a case under
  `conformance/format/` (FORMAT-1.0 §13).

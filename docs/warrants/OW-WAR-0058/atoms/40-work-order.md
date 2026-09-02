---
schema: oh.war/atom/v1
warrant_uuid: 01a06069-882d-7103-a07a-8e2d5c23bd12
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `SasRevision` in `openwarrant-core`: version, source, sha256, state
   (`proposed` | `accepted`), predecessor, §106 snapshot,
   `architecture_changing`, acceptance. `accept` consumes a proposal and
   returns an immutable accepted revision; there is no method that mutates
   an accepted one. An agent acceptance is refused; an architecture-changing
   acceptance without an `adr_ref` is refused (§101.3).
2. `Section106Diff`: added, removed, retitled; `is_architecture_changing`;
   `check_stability` refuses any removed id.
3. `docs/sas/revisions/<version>.toml` records, loaded by the repository; a
   malformed record is an error, never an absent one.
4. `war sas propose <version>`: computes the document's digest, snapshots
   §106, diffs against the predecessor, refuses instability, writes a
   proposed record. `war sas accept <version>` emits a request naming the
   digest and the diff; `--response <file>` ingests a human's acceptance
   through the authority register. `war sas diff <candidate.md>` shows the
   §106 change and refuses a removal. `war sas status` lists revisions.
5. `war check`: `sas.digest-drift` when the document's bytes do not match
   the latest revision's digest; `sas.unrecorded` (warn) when no revision
   exists; `sas.revision-malformed` for a record that will not validate.
6. `FormatBasis` gains `sas_revision` and `sas_digest`, absent when no
   revision is recorded so existing digests do not move for nothing, present
   once one is — and then every `workspace_basis_digest` moves once, in one
   commit, deliberately.
7. `war status` reads the latest revision into the Release axis: version,
   digest, and whether it is accepted or only proposed.
8. `0.1.0-draft.1` proposed, as the document stands.

## Frozen Surfaces

The SAS bytes. This Warrant reads them and proposes them; it changes no
character of the document. `DigestDomain::ALL` — fifteen. The
`composition_revision_digest` — a SAS field on the format basis moves the
workspace basis digest and nothing else, exactly as the schema-pack bump did.

## Premade Instructions

- Do not accept the revision. Propose it and stop.
- Do not add a status column to §106.
- The one-commit digest move is the last commit of the work, after every
  other file is final.

## Autonomy and Escalation

Tier T2. Escalate if the SAS's front-matter version and the proposed
version disagree.

## Rollback

Revert. Every workspace basis digest returns to its pre-revision value, and
the SAS returns to being pinned by prose.
